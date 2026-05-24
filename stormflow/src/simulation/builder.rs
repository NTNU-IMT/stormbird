
use std::fs;

use serde::{Serialize, Deserialize};

use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

use stormbird::actuator_line::builder::ActuatorLineBuilder;

use crate::actuator_line_interface::ActuatorLineInterface;

use crate::boundary_conditions::{BoundaryConditionBuilder, BoundaryConditions, BoundaryCondition};
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
    pub initial_velocity: SpatialVector,
    pub boundary_conditions: [[BoundaryConditionBuilder; 2]; 3],
    pub nr_interior_cells: [usize; 3],
    pub viscosity: Float,
    #[serde(default)]
    pub pressure_solver: PressureSolverBuilder,
    #[serde(default)]
    pub actuator_line: Option<ActuatorLineBuilder>,
    #[serde(default)]
    pub geometries: Vec<GeometryBuilder>
}

impl SimulationBuilder {
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
            self.nr_interior_cells
        );
        
        let total_nr_cells = grid.nr_extended_cells();

        let velocity = vec![self.initial_velocity; total_nr_cells];
        let velocity_org = vec![SpatialVector::default(); total_nr_cells];
        let velocity_star = vec![SpatialVector::default(); total_nr_cells];
        
        let body_force = vec![SpatialVector::default(); total_nr_cells];
        
        let mut boundary_conditions = BoundaryConditions::default();
        
        for i_a in 0..3 {
            for i_e in 0..2 {
                match self.boundary_conditions[i_a][i_e] {
                    BoundaryConditionBuilder::Slip => {
                        boundary_conditions.pressure[i_a][i_e] = BoundaryCondition::ZeroGradient;
                        boundary_conditions.velocity_x[i_a][i_e] = BoundaryCondition::ZeroGradient;
                        boundary_conditions.velocity_y[i_a][i_e] = BoundaryCondition::ZeroGradient;
                        boundary_conditions.velocity_z[i_a][i_e] = BoundaryCondition::ZeroGradient;
                    },
                    BoundaryConditionBuilder::NoSlip => {
                        boundary_conditions.pressure[i_a][i_e] = BoundaryCondition::ZeroGradient;
                        boundary_conditions.velocity_x[i_a][i_e] = BoundaryCondition::Value(0.0);
                        boundary_conditions.velocity_y[i_a][i_e] = BoundaryCondition::Value(0.0);
                        boundary_conditions.velocity_z[i_a][i_e] = BoundaryCondition::Value(0.0);
                    },
                    BoundaryConditionBuilder::Outlet => {
                        boundary_conditions.pressure[i_a][i_e] = BoundaryCondition::Value(0.0);
                        boundary_conditions.velocity_x[i_a][i_e] = BoundaryCondition::ZeroGradient;
                        boundary_conditions.velocity_y[i_a][i_e] = BoundaryCondition::ZeroGradient;
                        boundary_conditions.velocity_z[i_a][i_e] = BoundaryCondition::ZeroGradient;
                    },
                    BoundaryConditionBuilder::Inlet(inlet_velocity) => {
                        boundary_conditions.pressure[i_a][i_e] = BoundaryCondition::ZeroGradient;
                        boundary_conditions.velocity_x[i_a][i_e] = BoundaryCondition::Value(inlet_velocity[0]);
                        boundary_conditions.velocity_y[i_a][i_e] = BoundaryCondition::Value(inlet_velocity[1]);
                        boundary_conditions.velocity_z[i_a][i_e] = BoundaryCondition::Value(inlet_velocity[2]);
                    }
                }
            }
        }
        
        let pressure_solver = self.pressure_solver.build(
            &grid,
            &boundary_conditions
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
        let signed_distance_function = Geometry::signed_distance_function_on_grid(
            &geometries, &grid
        );
        
        Simulation {
            velocity,
            velocity_org,
            velocity_star,
            body_force,
            boundary_conditions,
            pressure_solver,
            grid,
            viscosity: self.viscosity,
            density: 1.0,
            actuator_line,
            signed_distance_function
        }
    }
}
