
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
    
    /// Time step
    #[arg(short, long)]
    time_step: Float,
    
    /// End time
    #[arg(short, long)]
    end_time: Float
}


pub fn main() -> Result<(), Error> {
    let args = Args::parse();
    
    let sim_builder = SimulationBuilder::from_json_file(&args.file_path)?;
    
    let mut sim = sim_builder.build();
    
    sim.initialize_after_build();
    
    let mut time = 0.0;
    
    while time < args.end_time {
        println!("Running time {}", time);
        sim.do_step(time, args.time_step);
        
        time += args.time_step;
    }
    
    sim.export_fields_as_vtk("sim_result.vtk", true);
    
    Ok(())
}
