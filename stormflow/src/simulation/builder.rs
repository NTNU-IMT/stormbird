
use std::fs;

use serde::{Serialize, Deserialize};

use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

use stormbird::actuator_line::builder::ActuatorLineBuilder;

use crate::actuator_line_interface::ActuatorLineInterface;

use crate::boundary_conditions::{BoundaryConditionBuilder, BoundaryConditions, BoundaryCondition};
use crate::grid::{Grid, INTERIOR_OFFSET};
use crate::simulation::Simulation;
use crate::geometry::Geometry;
use crate::pressure_solver::{
    PressureSolver,
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
    pub geometries: Vec<Geometry>
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
        let domain_length = self.domain_end_point - self.domain_start_point;
        
        let cell_length = SpatialVector([
            domain_length[0] / self.nr_interior_cells[0] as Float,
            domain_length[1] / self.nr_interior_cells[1] as Float,
            domain_length[2] / self.nr_interior_cells[2] as Float,
        ]);
        
        let nr_extended_cells = [
            self.nr_interior_cells[0] + 2 * INTERIOR_OFFSET,
            self.nr_interior_cells[1] + 2 * INTERIOR_OFFSET,
            self.nr_interior_cells[2] + 2 * INTERIOR_OFFSET,
        ];
        
        let total_nr_cells = nr_extended_cells[0] * nr_extended_cells[1] * nr_extended_cells[2];
        
        let pressure = vec![0.0; total_nr_cells];

        let velocity = vec![self.initial_velocity; total_nr_cells];
        let velocity_org = vec![SpatialVector::default(); total_nr_cells];
        let velocity_star = vec![SpatialVector::default(); total_nr_cells];
        
        let body_force = vec![SpatialVector::default(); total_nr_cells];
        
        let grid = Grid{
            start_point: self.domain_start_point,
            cell_length,
            nr_extended_cells
        };
        
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
        
        let (_, pressure_fixed_rhs) = PressureSolver::poisson_matrix_and_rhs(
            &grid, 
            &boundary_conditions
        );
        
        let pressure_solver = self.pressure_solver.build(
            &grid,
            &boundary_conditions
        );
        
        let actuator_line = if let Some(builder) = &self.actuator_line {
            Some(
                ActuatorLineInterface{
                    model: builder.build(),
                    dominating_line_indices: Vec::new(),
                    summed_projection_weights: Vec::new()
                }
            )
        } else {
            None
        };
        
        Simulation {
            pressure, 
            velocity,
            velocity_org,
            velocity_star,
            body_force,
            boundary_conditions,
            pressure_fixed_rhs,
            pressure_solver,
            grid,
            viscosity: self.viscosity,
            density: 1.0,
            actuator_line,
            geometries: self.geometries.clone()
        }
    }
}
