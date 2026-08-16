
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
    GeometryBuilder
};
use crate::pressure_solver::{
    builder::PressureSolverBuilder,
    boundary_conditions::PressureBoundaryConditions
};

use crate::velocity_solver::{
    VelocitySolver, boundary_condisitions::VelocityBoundaryConditions,
    slip_mirror_stencils::{SlipMirrorStencils, SlipMirrorInterpolationOrder, SLIP_MIRROR_REACH_CELLS}
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
}

impl SimulationBuilder {
    pub fn default_effective_viscosity() -> Float {0.0001}
    
    pub fn from_json_str(input: &str) -> Result<Self, Error> {
        let out = serde_json::from_str(input)?;
        
        Ok(out)
    }
    
    pub fn from_json_file(file_path: &str) -> Result<Self, Error> {
        let file_content = fs::read_to_string(file_path)?;
        
        Self::from_json_str(&file_content)
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
        
        let total_nr_cells = grid.nr_extended_cells();

        let pressure_boundary_conditions = PressureBoundaryConditions::new_from_up_direction(
            self.wind_environment.up_direction
        );

        let velocity_boundary_conditions = VelocityBoundaryConditions::new(
            &self.wind_environment,
            &self.wind_condition,
            self.linear_velocity,
            self.wind_environment.up_direction
        );

        let velocity = velocity_boundary_conditions.initial_velocity(&grid);
        let velocity_org = velocity.clone();
        let velocity_star = velocity.clone();
        let body_force = vec![SpatialVector::default(); total_nr_cells];

        let mut max_dx = 0.0;
        for axis_index in 0..3 {
            if grid.cell_length[axis_index] > max_dx {
                max_dx = grid.cell_length[axis_index];
            }
        }

        let actuator_line = self.actuator_line.as_ref().map(
            |builder| ActuatorLineInterface::new(builder.build(), &grid)
        );

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

        let pressure_solver = self.pressure_solver.build(
            &grid,
            &pressure_boundary_conditions,
            &slip_geometries
        );

        println!("Calculating SDF");
        let signed_distance_function = Geometry::signed_distance_function_on_extended_grid(
            &geometries, &grid
        );

        let signed_distance_function_slip = Geometry::signed_distance_function_on_extended_grid(
            &slip_geometries, &grid
        );

        let normals_slip_surfaces = Geometry::geometry_normals_on_extended_grid(
            &slip_geometries, &grid, 0.1
        );

        let slip_epsilon = 4.0 * max_dx;

        println!("Building slip-mirror stencils");
        let slip_mirror_stencils = SlipMirrorStencils::build(
            &grid,
            &signed_distance_function_slip,
            &normals_slip_surfaces,
            slip_epsilon,
            SLIP_MIRROR_REACH_CELLS * max_dx,
            self.slip_velocity_interpolation_order,
        );

        let velocity_solver = VelocitySolver {
            velocity,
            velocity_org,
            velocity_star,
            body_force,
            signed_distance_function,
            signed_distance_function_slip,
            normals_slip_surfaces,
            slip_mirror_stencils,
            boundary_conditions: velocity_boundary_conditions,
            no_slip_epsilon: 2.0 * max_dx,
            viscosity: self.effective_viscosity,
            density: 1.0,
        };
        
        Simulation {
            grid,
            velocity_solver,
            pressure_solver,
            actuator_line,
            solver_settings: self.solver_settings.clone()
        }
    }
}
