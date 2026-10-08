use stormath::type_aliases::Float;

pub mod io;
pub mod builder;

use crate::grid::Grid;

use crate::actuator_line_interface::ActuatorLineInterface;

use crate::pressure_solver::PressureSolver;
use crate::velocity_solver::VelocitySolver;
use crate::turbulence_solver::TurbulenceSolver;
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
    /// Only present when a turbulence model is used
    pub turbulence_solver: Option<TurbulenceSolver>,
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

        if let Some(turbulence_solver) = &mut self.turbulence_solver {
            turbulence_solver.initialize(&self.grid, &mut self.velocity_solver);
        }

        println!();
    }

    /// The number of velocity components that were clipped by the velocity limiter during the last
    /// time step, summed over all the inner iterations. Always zero if the limiter is not used (see
    /// `VelocitySolverBuilder::max_velocity_factor`).
    pub fn nr_limited_velocity_values(&self) -> usize {
        self.velocity_solver.nr_limited_velocity_values()
    }

    pub fn time_step_from_courant_number(&self, courant_number: Float) -> Float {
        let max_velocity = self.velocity_solver.max_velocity();

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
                self.solve_pressure(time_step);
                //println!("Project pressure time: {:.?}", start_time.elapsed());
            }
 
            //let start_time = Instant::now();
            self.update_velocity(time_step);
            //println!("Update velocity time: {:.?}", start_time.elapsed());
        }

        if let Some(turbulence_solver) = &mut self.turbulence_solver {
            turbulence_solver.update(&self.grid, &mut self.velocity_solver, time_step);
        }

        //let start_time = Instant::now();
        self.run_actuator_line_model(time, time_step);
        //println!("Running actuator line model time: {:.?}", start_time.elapsed());
        
        //println!();
    }

    /// Computes the right hand side of the pressure equation from the velocity solver, and solves
    /// the pressure equation. The data is only moved between the host and the device when the two
    /// solvers run on different platforms.
    fn solve_pressure(&mut self, time_step: Float) {
        let grid = &self.grid;

        match (&mut self.velocity_solver, &mut self.pressure_solver) {
            (VelocitySolver::CPU(velocity_solver), PressureSolver::MultigridCPU(pressure_solver)) => {
                velocity_solver.compute_pressure_rhs(grid, time_step, &mut pressure_solver.rhs_at_levels[0]);
                pressure_solver.solve();
            },
            (VelocitySolver::CPU(velocity_solver), PressureSolver::MultigridGPU(pressure_solver)) => {
                velocity_solver.compute_pressure_rhs(grid, time_step, &mut pressure_solver.rhs);
                pressure_solver.solve();
            },
            (VelocitySolver::GPU(velocity_solver), PressureSolver::MultigridGPU(pressure_solver)) => {
                // The buffers are shared, so the rhs is written directly to the pressure solver,
                // and the result is read directly by the velocity solver.
                velocity_solver.compute_pressure_rhs(time_step);
                pressure_solver.solve_on_device();
            },
            (VelocitySolver::GPU(velocity_solver), PressureSolver::MultigridCPU(pressure_solver)) => {
                velocity_solver.compute_pressure_rhs(time_step);
                velocity_solver.read_pressure_rhs(&mut pressure_solver.rhs_at_levels[0]);
                pressure_solver.solve();
                velocity_solver.write_pressure(&pressure_solver.solution);
            },
        }
    }

    /// Adds the gradient of the latest pressure solution to the velocity
    fn update_velocity(&mut self, time_step: Float) {
        match &mut self.velocity_solver {
            VelocitySolver::CPU(velocity_solver) => {
                // The CPU version of the pressure solver always has the solution on the host. So 
                // does the GPU version, as it is solved with `MultigridGPU::solve` in this case.
                velocity_solver.update_velocity(
                    &self.grid,
                    self.pressure_solver.pressure_ref(),
                    time_step
                );
            },
            VelocitySolver::GPU(velocity_solver) => {
                velocity_solver.update_velocity(time_step);
            }
        }
    }

    pub fn run_actuator_line_model(&mut self, time: Float, time_step: Float) {        
        let Some(actuator_line) = self.actuator_line.as_mut() else {
            return;
        };

        if time < actuator_line.model.start_time {
            return;
        }

        // The actuator line model only needs the velocity at, and only sets the body force in, 
        // the cells close to the lines.
        let cell_velocities = self.velocity_solver.cell_centered_velocity_at_cells(
            &self.grid,
            &actuator_line.cell_indices_to_check
        );

        actuator_line.step_model(time, time_step, &self.grid, &cell_velocities);

        let body_force = actuator_line.compute_body_force(&cell_velocities);

        self.velocity_solver.set_body_force_at_cells(
            &actuator_line.cell_indices_to_check,
            &body_force
        );

        let _need_update = actuator_line.model.update_controller(time, time_step);
    }
}
