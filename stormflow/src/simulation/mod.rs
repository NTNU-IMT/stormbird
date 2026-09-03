use stormath::type_aliases::Float;

pub mod io;
pub mod builder;

use crate::grid::Grid;

use crate::actuator_line_interface::ActuatorLineInterface;

use crate::pressure_solver::PressureSolver;
use crate::velocity_solver::VelocitySolver;
use builder::SimulationBuilder;

use serde::{Serialize, Deserialize};

use crate::error::Error;

//use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolverSettings {
    #[serde(default="SolverSettings::default_nr_inner_iterations")]
    pub nr_inner_iterations: usize,
    #[serde(default="SolverSettings::default_solve_pressure_on_first_iteration")]
    pub solve_pressure_on_first_iteration: bool
}

impl SolverSettings {
    pub fn default_nr_inner_iterations() -> usize {2}
    pub fn default_solve_pressure_on_first_iteration() -> bool {true}
}

impl Default for SolverSettings {
    fn default() -> Self {
        Self {
            nr_inner_iterations: Self::default_nr_inner_iterations(),
            solve_pressure_on_first_iteration: Self::default_solve_pressure_on_first_iteration()
        }
    }
}

pub struct Simulation {
    pub grid: Grid,
    pub velocity_solver: VelocitySolver,
    pub pressure_solver: PressureSolver,
    pub actuator_line: Option<ActuatorLineInterface>,
    pub solver_settings: SolverSettings
}

impl Simulation {
    pub fn new_from_string(setup_string: &str) -> Result<Self, Error> {
        let builder = SimulationBuilder::new_from_string(setup_string)?;

        let mut sim = builder.build();

        sim.initialize_after_build();

        Ok(sim)
    }
    
    pub fn initialize_after_build(&mut self) {
        println!("Initializing after build");
        self.velocity_solver.initialize_after_build(&self.grid);

        println!();
    }

    pub fn time_step_from_courant_number(&self, courant_number: Float) -> Float {
        let mut max_velocity = 0.0;
        for i in 0..self.velocity_solver.velocity.len() {
            if self.velocity_solver.velocity[i].length() > max_velocity {
                max_velocity = self.velocity_solver.velocity[i].length();
            }
        }

        let cell_length = self.grid.cell_length;

        let mut min_cell_length = Float::INFINITY;
        for i in 0..3 {
            if cell_length[i] < min_cell_length {
                min_cell_length = cell_length[i];
            }
        }

        courant_number * min_cell_length / max_velocity
    }

    pub fn do_steps_until_end_time(&mut self, end_time: Float, courant_number: Float) {
        let mut time = 0.0;

        while time < end_time {
            let time_step = self.time_step_from_courant_number(courant_number);

            self.do_step(time, time_step);

            time += time_step
        }
    }

    pub fn do_step(&mut self, time: Float, time_step: Float) {      
        self.velocity_solver.initialize_before_step(&self.grid);

        for iteration in 0..self.solver_settings.nr_inner_iterations {
            //println!("Prediction {}", iteration+1);
            //let start_time = Instant::now();
            self.velocity_solver.update_velocity_star(&self.grid, time_step);
            //println!("Update velocity star time: {:.?}", start_time.elapsed());

            if iteration > 0 || 
                self.solver_settings.solve_pressure_on_first_iteration ||
                self.solver_settings.nr_inner_iterations == 1 {
                //let start_time = Instant::now();
                self.pressure_solver.calculate_rhs(
                    &self.grid, 
                    &self.velocity_solver.velocity_star, 
                    self.velocity_solver.density, 
                    time_step
                );
                //println!("Pressure rhs time: {:.?}", start_time.elapsed());
                
                //let start_time = Instant::now();
                self.pressure_solver.solve();
                //println!("Project pressure time: {:.?}", start_time.elapsed());
            }
 
            //let start_time = Instant::now();
            self.velocity_solver.update_velocity(
                &self.grid, 
                self.pressure_solver.pressure_ref(), 
                time_step
            );
            //println!("Update velocity time: {:.?}", start_time.elapsed());
        }

        //let start_time = Instant::now();
        self.run_actuator_line_model(time, time_step);
        //println!("Running actuator line model time: {:.?}", start_time.elapsed());
        
        //println!();
    }

    pub fn run_actuator_line_model(&mut self, time: Float, time_step: Float) {        
        if let Some(actuator_line) = self.actuator_line.as_mut() {
            actuator_line.step_model(
                time, 
                time_step, 
                &self.grid, 
                &self.velocity_solver.velocity
            );
        }

        if let Some(actuator_line) = &self.actuator_line {
            actuator_line.compute_body_force(
                &self.grid, 
                &self.velocity_solver.velocity, 
                self.velocity_solver.density, 
                &mut self.velocity_solver.body_force
            );
        }

        if let Some(actuator_line) = self.actuator_line.as_mut() {
            let _need_update = actuator_line.model.update_controller(time, time_step);
        }
    }
}
