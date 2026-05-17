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
    #[arg(short, long, default_value_t = 0.5)]
    courant_number: Float,
}


pub fn main() -> Result<(), Error> {
    let args = Args::parse();
    
    let sim_builder = SimulationBuilder::from_json_file(&args.file_path)?;
    
    let mut sim = sim_builder.build();
    
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
