
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

use crate::boundary_conditions::{
    pressure::PressureBoundaryConditions,
    velocity::VelocityBoundaryConditions
};
use crate::grid::Grid;
use crate::simulation::Simulation;
use crate::geometry::{
    Geometry,
    GeometryBuilder
};
use crate::pressure_solver::{
    builder::PressureSolverBuilder
};

use crate::error::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SimulationBuilder {
    pub domain_start_point: SpatialVector,
    pub domain_end_point: SpatialVector,
    pub wind_condition: WindCondition,
    pub linear_velocity: SpatialVector,
    pub grid_interior_shape: [usize; 3],
    #[serde(default)]
    pub actuator_line: Option<ActuatorLineBuilder>,
    #[serde(default)]
    pub geometries: Vec<GeometryBuilder>,
    #[serde(default="SimulationBuilder::default_effective_viscosity")]
    pub effective_viscosity: Float,
    #[serde(default)]
    pub wind_environment: WindEnvironment,
    #[serde(default)]
    pub pressure_solver: PressureSolverBuilder,
}

impl SimulationBuilder {
    pub fn default_effective_viscosity() -> Float {0.001}
    
    pub fn from_json_str(input: &str) -> Result<Self, Error> {
        let out = serde_json::from_str(input)?;
        
        Ok(out)
    }
    
    pub fn from_json_file(file_path: &str) -> Result<Self, Error> {
        let file_content = fs::read_to_string(file_path)?;
        
        Self::from_json_str(&file_content)
    }
    
    pub fn build(&self) -> Simulation {
        let grid = Grid::new(
            self.domain_start_point, 
            self.domain_end_point, 
            self.grid_interior_shape
        );
        
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
        
        let pressure_solver = self.pressure_solver.build(
            &grid,
            &pressure_boundary_conditions
        );
        
        let actuator_line = if let Some(builder) = &self.actuator_line {
            Some(
                ActuatorLineInterface::new(builder.build(), &grid)
            )
        } else {
            None
        };

        let mut geometries: Vec<Geometry> = Vec::new();

        for geo_builder in &self.geometries {
            geometries.push(
                geo_builder.build()
            )
        }

        println!("Calculating SDF");
        let signed_distance_function = Geometry::signed_distance_function_on_extended_grid(
            &geometries, &grid
        );
        
        Simulation {
            velocity,
            velocity_org,
            velocity_star,
            velocity_boundary_conditions,
            body_force,
            pressure_solver,
            grid,
            viscosity: self.effective_viscosity,
            density: 1.0,
            actuator_line,
            signed_distance_function
        }
    }
}
