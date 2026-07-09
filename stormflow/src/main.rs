use std::time::Instant;

use stormflow::{
    simulation::builder::SimulationBuilder,
    error::Error,
};

use stormath::type_aliases::Float;

use clap::Parser;

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
    wind_direction_deg: f32
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

    if args.section_models_internal_state.len() > 0 {
        if let Some(actuator_line) = &mut sim.actuator_line {
            actuator_line.model.line_force_model.set_section_models_internal_state(
                &args.section_models_internal_state
            );
        }
    }

    sim.initialize_after_build();
    
    let mut time = 0.0;

    let start_time = Instant::now();
    
    while time < args.end_time {
        let time_step = sim.time_step_from_courant_number(args.courant_number);
        
        println!("Running time {}, with time step {}", time, time_step);
        sim.do_step(time, time_step);
        
        time += time_step;
    }

    let duration = start_time.elapsed();

    println!("Total simulation time {:?} s", duration.as_secs());
    
    sim.export_fields_as_vtk("sim_result.vtk", true);
    
    Ok(())
}
