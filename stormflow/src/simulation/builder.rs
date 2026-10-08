
use std::fs;
use std::collections::HashSet;

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
    VelocitySolver, VelocitySolverSetup, boundary_condisitions::VelocityBoundaryConditions,
    slip_mirror_stencils::{
        SlipMirrorStencils, SlipMirrorInterpolationOrder, SLIP_MIRROR_REACH_CELLS, mirror_interior_corrections
    },
    no_slip_corrections::NoSlipCorrections,
    wall_model::{NoSlipWallTreatment, WallStressEntries},
    cpu::VelocitySolverCPU,
    gpu::{VelocitySolverGPU, SharedPressureBuffers},
};
use crate::pressure_solver::PressureSolver;
use crate::turbulence_solver::{
    TurbulenceSolver,
    builder::TurbulenceSolverBuilder,
    cpu::TurbulenceSolverCPU,
    gpu::TurbulenceSolverGPU,
};
use crate::gpu_interface::{
    ComputePlatform,
    context::GpuContext
};

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
    #[serde(default)]
    pub pressure_solver: PressureSolverBuilder,
    #[serde(default)]
    pub solver_settings: SolverSettings,
    /// Interpolation order for the slip-wall *velocity* mirror correction specifically (see
    /// `SlipMirrorInterpolationOrder`) — independent of `pressure_solver`'s equivalent pressure
    /// setting, though you'll usually want to set both to the same order for matching accuracy at
    /// the slip wall on both fields. Defaults to 4th order (`Tricubic`, matching the rest of the
    /// solver); switch to `Trilinear` (2nd order) for thin walls, where the tricubic stencil's
    /// wider reach is more likely to pull an image point in from the wrong side of a nearby
    /// second surface.
    #[serde(default)]
    pub slip_velocity_interpolation_order: SlipMirrorInterpolationOrder,
    /// Optional "override" of the boundary conditions, to be able to set some to slip walls
    #[serde(default)]
    pub slip_wall_boundary_override: [[bool; 2]; 3],
    /// How the velocity solver treats the no-slip geometries: with the data immersion (default),
    /// or with the wall model, which is a slip condition together with the wall shear stress from
    /// the log-law (see `velocity_solver::wall_model`).
    #[serde(default)]
    pub no_slip_wall_treatment: NoSlipWallTreatment,
    /// Where to execute the velocity solver. Independent of where the pressure solver is executed
    /// (see `pressure_solver`), but the fewest transfers between the host and the device happen 
    /// when both are on the same platform.
    #[serde(default)]
    pub velocity_solver_compute_platform: ComputePlatform,
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

        let mut max_dx = 0.0;
        for axis_index in 0..3 {
            if grid.cell_length[axis_index] > max_dx {
                max_dx = grid.cell_length[axis_index];
            }
        }

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
        let gpu_context = if self.velocity_solver_compute_platform.is_gpu() || 
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

        // The blending width of the no-slip correction, which the turbulence solver's wall
        // treatment must be consistent with
        let no_slip_epsilon = 2.0 * max_dx;

        let turbulence_solver_setup = self.turbulence.as_ref().map(|builder| {
            builder.build_setup(
                &grid,
                &velocity_boundary_conditions,
                &self.wind_environment,
                &no_slip_walls,
                &slip_walls,
                self.effective_viscosity,
                no_slip_epsilon
            )
        });

        let use_eddy_viscosity = turbulence_solver_setup.is_some();

        let use_wall_model = self.no_slip_wall_treatment == NoSlipWallTreatment::WallModel;

        // With the wall model, the no-slip geometries get the same mirror correction as the slip
        // geometries, so the stencils are built from the union of both
        let mirror_geometries: Vec<Geometry> = if use_wall_model {
            slip_walls.geometries.iter().chain(no_slip_walls.geometries.iter()).cloned().collect()
        } else {
            slip_walls.geometries.clone()
        };

        let signed_distance_function_mirror: Vec<Float> = if use_wall_model {
            slip_walls.signed_distance_function.iter()
                .zip(&no_slip_walls.signed_distance_function)
                .map(|(slip, no_slip)| slip.min(*no_slip))
                .collect()
        } else {
            slip_walls.signed_distance_function.clone()
        };

        let wall_stress = if use_wall_model {
            println!("Building wall model");
            WallStressEntries::build(
                &grid,
                &no_slip_walls.geometries,
                &no_slip_walls.signed_distance_function,
                &slip_walls.signed_distance_function,
            )
        } else {
            WallStressEntries::default()
        };

        let no_slip_geometries = no_slip_walls.geometries.clone();
        let signed_distance_function = no_slip_walls.signed_distance_function;
        let signed_distance_function_slip = slip_walls.signed_distance_function;

        let slip_reach_distance = SLIP_MIRROR_REACH_CELLS * max_dx;

        // The normals are only computed where the slip-mirror stencils use them, and are zero
        // everywhere else
        let normals_slip_surfaces = Geometry::geometry_normals_on_extended_grid(
            &mirror_geometries, &grid, 0.1,
            &SlipMirrorStencils::cells_needing_normals(
                &grid, &signed_distance_function_mirror, slip_reach_distance
            )
        );

        let slip_epsilon = 4.0 * max_dx;

        println!("Building slip-mirror stencils");
        let mut slip_mirror_stencils = SlipMirrorStencils::build(
            &grid,
            &signed_distance_function_mirror,
            &normals_slip_surfaces,
            slip_epsilon,
            slip_reach_distance,
            self.slip_velocity_interpolation_order,
        );

        // The velocity is set to zero deep inside the geometries with a mirror correction, and where
        // the mirror correction is not well defined, see `mirror_interior_corrections`
        println!("Building corrections for the interior of the mirror geometries");
        let all_geometries: Vec<Geometry> = slip_walls.geometries.iter()
            .chain(no_slip_geometries.iter())
            .cloned()
            .collect();

        let (mirror_interior, zeroed_faces) = mirror_interior_corrections(
            &grid,
            &mirror_geometries,
            &all_geometries,
            &signed_distance_function_mirror,
            slip_reach_distance,
        );

        let zeroed_faces: HashSet<(usize, usize)> = zeroed_faces.into_iter().collect();

        for (axis, entries) in slip_mirror_stencils.entries.iter_mut().enumerate() {
            entries.retain(|entry| !zeroed_faces.contains(&(entry.cell_index, axis)));
        }

        let no_slip_corrections = if use_wall_model {
            mirror_interior
        } else {
            println!("Building no-slip corrections");
            NoSlipCorrections::build(
                &grid,
                &signed_distance_function,
                no_slip_epsilon,
            ).merged(mirror_interior)
        };

        let velocity_solver_setup = VelocitySolverSetup {
            signed_distance_function,
            signed_distance_function_slip,
            normals_slip_surfaces,
            no_slip_corrections,
            slip_mirror_stencils,
            wall_stress,
            boundary_conditions: velocity_boundary_conditions,
            viscosity: self.effective_viscosity,
            density,
        };

        let velocity_solver = match self.velocity_solver_compute_platform {
            ComputePlatform::CPU => VelocitySolver::CPU(
                VelocitySolverCPU::new(velocity_solver_setup, velocity, use_eddy_viscosity)
            ),
            ComputePlatform::GPU => {
                let shared_pressure_buffers = match &pressure_solver {
                    PressureSolver::MultigridGPU(solver) => Some(SharedPressureBuffers {
                        rhs: solver.rhs_buffer().clone(),
                        pressure: solver.solution_buffer().clone(),
                    }),
                    PressureSolver::MultigridCPU(_) => None,
                };

                VelocitySolver::GPU(
                    VelocitySolverGPU::new(
                        gpu_context.expect("A GPU context is always created for a GPU velocity solver"),
                        &grid,
                        velocity_solver_setup,
                        &velocity,
                        shared_pressure_buffers,
                        use_eddy_viscosity
                    )
                )
            }
        };
        
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
