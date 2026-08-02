use std::io::Read;
use std::time::Instant;

use stormflow::{
    simulation::builder::SimulationBuilder,
    error::Error,
};

use stormath::type_aliases::Float;

use clap::Parser;
use console::Term;
use gag::BufferRedirect;

/// Format a progress bar string
fn format_progress_bar(
    current: f64,
    total: f64,
    elapsed: std::time::Duration,
    width: usize,
) -> String {
    let fraction = (current / total).clamp(0.0, 1.0);
    let percent = (fraction * 100.0) as u32;
    let filled = (width as f64 * fraction) as usize;
    let empty = width - filled;
    
    // Estimate ETA
    let eta = if fraction > 0.0 {
        let total_estimated = elapsed.as_secs_f64() / fraction;
        let remaining = total_estimated - elapsed.as_secs_f64();
        format!("{:.0}s", remaining)
    } else {
        "--".to_string()
    };
    
    format!(
        "[{:02}:{:02}:{:02}] [{}{}] {:3}% (ETA: {}) | t = {:.4} / {:.4}",
        elapsed.as_secs() / 3600,
        (elapsed.as_secs() % 3600) / 60,
        elapsed.as_secs() % 60,
        "=".repeat(filled),
        " ".repeat(empty),
        percent,
        eta,
        current,
        total
    )
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to the settings file
    #[arg(short, long)]
    file_path: String,

    /// End time
    #[arg(short, long)]
    end_time: Float,
    
    /// Time step
    #[arg(short, long, default_value_t = 0.75)]
    courant_number: Float,

    /// Number of cores
    #[arg(short, long, default_value_t = 0)]
    nr_of_cores: usize,

    /// New control variables
    #[arg(short, long, allow_negative_numbers = true)]
    section_models_internal_state: Vec<f32>,

    /// New wind direction in degrees
    #[arg(short, long, default_value_t = -9999.0, allow_negative_numbers = true)]
    wind_direction_deg: f32,
    
}


pub fn main() -> Result<(), Error> {
    let args = Args::parse();

    if args.nr_of_cores > 0 {
        rayon::ThreadPoolBuilder::new()
            .num_threads(args.nr_of_cores)
            .build_global()
            .unwrap();
    }
    
    let mut sim_builder = SimulationBuilder::from_json_file(&args.file_path)?;

    if args.wind_direction_deg > -9999.0 {
        sim_builder.wind_condition.direction_coming_from = args.wind_direction_deg.to_radians();
    }
    
    let mut sim = sim_builder.build();

    let new_internal_states: bool = !args.section_models_internal_state.is_empty();

    if new_internal_states && let Some(actuator_line) = &mut sim.actuator_line {
        actuator_line.model.line_force_model.set_section_models_internal_state(
            &args.section_models_internal_state
        );
    }

    sim.initialize_after_build();
    
    let mut time = 0.0;

    let start_time = Instant::now();

    // Set up terminal for output management
    let term = Term::buffered_stdout();
    
    // Track lines printed in previous iteration (for cursor repositioning)
    let mut lines_printed: usize;
    let mut prev_lines_printed: usize = 0;
    
    while time < args.end_time {
        // Move cursor back to start of our display area (don't clear, just reposition)
        if prev_lines_printed > 0 {
            let _ = term.move_cursor_up(prev_lines_printed);
        }
        lines_printed = 0;
        
        let time_step = sim.time_step_from_courant_number(args.courant_number);
        let elapsed = start_time.elapsed();
        
        // Get terminal width for padding (to fully overwrite previous content)
        let term_width = term.size().1 as usize;
        
        // Print progress bar (1 line) - pad to terminal width to overwrite old content
        let progress_line = format_progress_bar(time as f64, args.end_time as f64, elapsed, 40);
        let padded_progress = format!("{:<width$}", progress_line, width = term_width);
        let _ = term.write_line(&padded_progress);
        lines_printed += 1;
        
        // Print time step info (1 line)
        let time_step_line = format!("Time step: {:.6}", time_step);
        let padded_time_step = format!("{:<width$}", time_step_line, width = term_width);
        let _ = term.write_line(&padded_time_step);
        lines_printed += 1;
        
        // Flush to ensure progress bar is visible before do_step
        let _ = term.flush();
        
        // Capture stdout from do_step so we can count lines
        let mut captured_output = String::new();
        {
            let mut buf = BufferRedirect::stdout().expect("Failed to redirect stdout");
            sim.do_step(time, time_step);
            buf.read_to_string(&mut captured_output).ok();
        }
        
        // Print captured output and count lines
        if !captured_output.is_empty() {
            // Print each line, padded to overwrite previous content
            let output = captured_output.trim_end_matches('\n');
            for line in output.lines() {
                let padded_line = format!("{:<width$}", line, width = term_width);
                let _ = term.write_line(&padded_line);
                lines_printed += 1;
            }
        }
        
        // If we printed fewer lines than last time, clear the extra lines
        if lines_printed < prev_lines_printed {
            for _ in 0..(prev_lines_printed - lines_printed) {
                // Write a blank line to clear leftover content
                let _ = term.write_line(&" ".repeat(term_width));
            }
        }
        
        // Flush to ensure output is visible
        let _ = term.flush();
        
        prev_lines_printed = lines_printed;
        time += time_step;
    }

    // Move cursor back and overwrite with final state
    if prev_lines_printed > 0 {
        let _ = term.move_cursor_up(prev_lines_printed);
    }
    
    let term_width = term.size().1 as usize;
    
    // Final progress bar (complete)
    let elapsed = start_time.elapsed();
    let progress_line = format_progress_bar(args.end_time as f64, args.end_time as f64, elapsed, 40);
    let final_line = format!("{} - Complete!", progress_line);
    let _ = term.write_line(&format!("{:<width$}", final_line, width = term_width));
    
    // Clear any remaining lines from previous output
    for _ in 1..prev_lines_printed {
        let _ = term.write_line(&" ".repeat(term_width));
    }
    
    // Move back up to just after the completion line, then clear those blank lines
    if prev_lines_printed > 1 {
        let _ = term.move_cursor_up(prev_lines_printed - 1);
        let _ = term.clear_to_end_of_screen();
    }
    
    let _ = term.flush();

    let duration = start_time.elapsed();

    println!("Total simulation time {:?} s", duration.as_secs());
    
    sim.export_fields_as_vtk("sim_result.vtk", true);
    
    Ok(())
}
