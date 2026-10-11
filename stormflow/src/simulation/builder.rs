
use std::fs;

use serde::{Serialize, Deserialize};

use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

use stormbird::{
    wind::{
        environment::WindEnvironment,
        wind_condition::WindCondition
    },
    actuator_line::builder::ActuatorLineBuilder
};

use crate::actuator_line_interface::ActuatorLineInterface;

use crate::grid::{
    builder::GridBuilder,
};
use crate::simulation::{Simulation, SolverSettings};
use crate::geometry::{
    Geometry,
    GeometryBuilder,
    WallGeometries
};
use crate::pressure_solver::{
    builder::PressureSolverBuilder,
    boundary_conditions::PressureBoundaryConditions
};

use crate::velocity_solver::{
    VelocitySolver,
    builder::VelocitySolverBuilder,
    boundary_condisitions::VelocityBoundaryConditions,
};
use crate::turbulence_solver::{
    TurbulenceSolver,
    builder::TurbulenceSolverBuilder,
    cpu::TurbulenceSolverCPU,
    gpu::TurbulenceSolverGPU,
};
use crate::gpu_interface::context::GpuContext;

use crate::error::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SimulationBuilder {
    pub grid: GridBuilder,
    pub wind_condition: WindCondition,
    pub linear_velocity: SpatialVector,
    #[serde(default)]
    pub actuator_line: Option<ActuatorLineBuilder>,
    #[serde(default)]
    pub geometries: Vec<GeometryBuilder>,
    #[serde(default)]
    pub slip_geometries: Vec<GeometryBuilder>,
    #[serde(default="SimulationBuilder::default_effective_viscosity")]
    pub effective_viscosity: Float,
    #[serde(default)]
    pub wind_environment: WindEnvironment,
    /// Settings for the velocity solver
    #[serde(default)]
    pub velocity_solver: VelocitySolverBuilder,
    /// Settings for the pressure solver
    #[serde(default)]
    pub pressure_solver: PressureSolverBuilder,
    #[serde(default)]
    pub solver_settings: SolverSettings,
    /// Optional "override" of the boundary conditions, to be able to set some to slip walls
    #[serde(default)]
    pub slip_wall_boundary_override: [[bool; 2]; 3],
    /// Optional RANS turbulence model. The turbulence solver always runs on the same platform as
    /// the velocity solver. `effective_viscosity` is used as the molecular viscosity when a
    /// turbulence model is used.
    #[serde(default)]
    pub turbulence: Option<TurbulenceSolverBuilder>,
}

impl SimulationBuilder {
    pub fn default_effective_viscosity() -> Float {0.0001}
    
    pub fn new_from_string(input: &str) -> Result<Self, Error> {
        let out = serde_json::from_str(input)?;
        
        Ok(out)
    }
    
    pub fn from_json_file(file_path: &str) -> Result<Self, Error> {
        let file_content = fs::read_to_string(file_path)?;
        
        Self::new_from_string(&file_content)
    }
    
    pub fn build(&self) -> Simulation {
        let grid = if let Some(actuator_line_builder) = &self.actuator_line {
            self.grid.build_from_line_force_model_builder(
                &actuator_line_builder.line_force_model
            )
        } else {
            self.grid.build_from_internal_length()
                .unwrap_or_else(|err| panic!("{}", err))
        };

        println!("Interior shape of the grid: {:?}", &grid.interior_shape);
        
        // The density of the fluid. The actuator line model is synced to this value, so that the
        // forces it projects are consistent with the flow.
        let density: Float = 1.0;

        let velocity_boundary_conditions = VelocityBoundaryConditions::new(
            &self.wind_environment,
            &self.wind_condition,
            self.linear_velocity,
            self.slip_wall_boundary_override,
            &grid
        );

        let pressure_boundary_conditions = PressureBoundaryConditions::new_from_velocity_boundary_conditions(
            &velocity_boundary_conditions
        );

        let velocity = velocity_boundary_conditions.initial_velocity(&grid);

        let actuator_line = self.actuator_line.as_ref().map(|builder| {
            let mut model = builder.build();

            if model.line_force_model.density != density {
                println!(
                    "Note: the density of the actuator line model ({}) is replaced by the density \
                     of the flow solver ({})",
                    model.line_force_model.density, density
                );

                model.line_force_model.density = density;
            }

            ActuatorLineInterface::new(model, &grid)
        });

        let mut geometries: Vec<Geometry> = Vec::new();

        for geo_builder in &self.geometries {
            geometries.push(
                geo_builder.build()
            )
        }

        let mut slip_geometries: Vec<Geometry> = Vec::new();

        for geo_builder in &self.slip_geometries {
            slip_geometries.push(
                geo_builder.build()
            )
        }

        // The signed distance functions are computed once, and shared by the pressure and the
        // velocity solver
        println!("Calculating SDF");
        let no_slip_walls = WallGeometries::new(geometries, &grid);
        let slip_walls = WallGeometries::new(slip_geometries, &grid);

        // One context shared by all solvers on the GPU, so that they can share buffers
        let gpu_context = if self.velocity_solver.compute_platform.is_gpu() || 
            self.pressure_solver.compute_platform().is_gpu() {
            Some(GpuContext::new())
        } else {
            None
        };

        let pressure_solver = self.pressure_solver.build(
            &grid,
            &pressure_boundary_conditions,
            &slip_walls,
            &no_slip_walls,
            gpu_context.as_ref()
        );

        // The turbulence solver's wall treatment must be consistent with the blending width of the
        // no-slip correction
        let turbulence_solver_setup = self.turbulence.as_ref().map(|builder| {
            builder.build_setup(
                &grid,
                &velocity_boundary_conditions,
                &self.wind_environment,
                &no_slip_walls,
                &slip_walls,
                self.effective_viscosity,
                self.velocity_solver.no_slip_epsilon(&grid)
            )
        });

        let velocity_solver = self.velocity_solver.build(
            &grid,
            velocity_boundary_conditions,
            velocity,
            no_slip_walls,
            slip_walls,
            self.effective_viscosity,
            density,
            gpu_context,
            &pressure_solver,
            turbulence_solver_setup.is_some(),
        );

        // The turbulence solver runs on the same platform as the velocity solver, and shares the
        // velocity and the eddy viscosity directly with it on the GPU
        let turbulence_solver = turbulence_solver_setup.map(|setup| match &velocity_solver {
            VelocitySolver::CPU(_) => TurbulenceSolver::CPU(
                TurbulenceSolverCPU::new(setup, &grid)
            ),
            VelocitySolver::GPU(velocity_solver) => TurbulenceSolver::GPU(
                TurbulenceSolverGPU::new(
                    velocity_solver.context().clone(),
                    &grid,
                    setup,
                    velocity_solver.velocity_buffer().clone(),
                    velocity_solver.eddy_viscosity_buffer()
                        .expect("The velocity solver is created with an eddy viscosity")
                        .clone()
                )
            ),
        });

        Simulation {
            grid,
            velocity_solver,
            pressure_solver,
            turbulence_solver,
            actuator_line,
            solver_settings: self.solver_settings.clone()
        }
    }
}
