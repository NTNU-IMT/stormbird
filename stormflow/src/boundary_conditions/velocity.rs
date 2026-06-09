
use stormath::spatial_vector::SpatialVector;

use stormbird::wind::{
    environment::WindEnvironment,
    wind_condition::WindCondition,
};

use crate::grid::Grid;

#[derive(Debug, Clone, Copy)]
pub enum VelocityBoundaryCondition {
    ZeroGradient,
    InletOutlet,
    SlipWall,
}

#[derive(Debug, Clone)]
pub struct VelocityBoundaryConditions {
    pub wind_environment: WindEnvironment,
    pub wind_condition: WindCondition,
    pub linear_velocity: SpatialVector,
    pub face_conditions: [[VelocityBoundaryCondition; 2]; 3]
}

impl VelocityBoundaryConditions {
    pub fn new(
        wind_environment: &WindEnvironment, 
        wind_condition: &WindCondition, 
        linear_velocity: SpatialVector, 
        up_direction: SpatialVector
    ) -> Self {
        let mut face_conditions = [[VelocityBoundaryCondition::InletOutlet; 2]; 3];

        let mut up_axis: usize = 0;

        if up_direction[1].abs() > up_direction[0].abs()  && 
            up_direction[1].abs() > up_direction[2].abs() {
            up_axis = 1;
        } else if up_direction[2].abs() > up_direction[0].abs()  && 
            up_direction[2].abs() > up_direction[1].abs() {
            up_axis = 2;
        }

        face_conditions[up_axis][0] = VelocityBoundaryCondition::SlipWall;
        face_conditions[up_axis][1] = VelocityBoundaryCondition::ZeroGradient;

        Self {
            wind_environment: wind_environment.clone(),
            wind_condition: wind_condition.clone(),
            linear_velocity,
            face_conditions
        }
    }

    pub fn velocity_at_point(&self, point: SpatialVector) -> SpatialVector {
        self.wind_environment.steady_apparent_wind_velocity_vector_at_location(
            &self.wind_condition, point, self.linear_velocity
        )
    }

    pub fn initial_velocity(&self, grid: &Grid) -> Vec<SpatialVector> {
        let nr_extended_cells = grid.nr_extended_cells();

        (0..nr_extended_cells).into_iter().map(|i_flat_extended| {
            let extended_indices = grid.extended_indices_from_flat_index(i_flat_extended);
            let interior_indices = grid.interior_indices_from_extended_indices(extended_indices);

            let mut velocity = SpatialVector::default();
            let cell_center = grid.cell_center(interior_indices);

            for axis_index in 0..3 {
                let mut face_point = cell_center;
                face_point[axis_index] += 0.5 * grid.cell_length[axis_index]; // TODO: check if this is consistent over all the code...

                let face_velocity = self.velocity_at_point(face_point);

                velocity[axis_index] = face_velocity[axis_index]
            }

            velocity
        }).collect()
    }
    
    pub fn set_ghost_cells(&self, grid: &Grid, velocity: &mut [SpatialVector]) {
        let [nx, ny, nz] = grid.extended_shape.clone();
        
        for axis_index in 0..3 {
            let axis_length = match axis_index {
                0 => nx,
                1 => ny,
                2 => nz,
                _ => panic!("Axis index larger than 2")
            };
            
            let (n1, n2) = match axis_index {
                0 => (ny, nz),
                1 => (nx, nz),
                2 => (nx, ny),
                _ => panic!("Axis index larger than 2")
            };
            
            for face_index in 0..2 {
                for i_1 in 0..n1 {
                    for i_2 in 0..n2 {
                        let mut indices_current = [0, 0, 0];
                        let mut indices_neighbor = [0, 0, 0];
                        
                        if face_index == 0 {
                            indices_current[axis_index] = 0;
                            indices_neighbor[axis_index] = 1;
                        } else {
                            indices_current[axis_index] = axis_length-1;
                            indices_neighbor[axis_index] = axis_length-2;
                        }
                        
                        match axis_index {
                            0 => {     
                                indices_current[1] = i_1;
                                indices_neighbor[1] = i_1;
                                indices_current[2] = i_2;
                                indices_neighbor[2] = i_2;
                            },
                            1 => {
                                indices_current[0] = i_1;
                                indices_neighbor[0] = i_1;
                                indices_current[2] = i_2;
                                indices_neighbor[2] = i_2;
                            },
                            2 => {
                                indices_current[0] = i_1;
                                indices_neighbor[0] = i_1;
                                indices_current[1] = i_2;
                                indices_neighbor[1] = i_2;
                            },
                            _ => panic!("Axis index larger than 2")
                        }
                        
                        let flat_index_current = grid.flat_index_on_extended_grid(indices_current);
                        let flat_index_neighbor = grid.flat_index_on_extended_grid(indices_neighbor);
                        
                        match self.face_conditions[axis_index][face_index] {
                            VelocityBoundaryCondition::InletOutlet => {
                                // Check the direction of the flow in the neighbor cell
                                let neighbor_axis_flow = velocity[flat_index_neighbor][axis_index];

                                let inflow = if face_index == 1 {
                                    if neighbor_axis_flow > 0.0 {
                                        true
                                    } else {
                                        false
                                    }
                                } else {
                                    if neighbor_axis_flow < 0.0 {
                                        true
                                    } else {
                                        false
                                    }
                                };

                                // Set the values if inflow, otherwise assume zero gradient
                                if inflow {
                                    for c in 0..3 {
                                        let mut face_point = grid.cell_center(
                                            grid.interior_indices_from_extended_indices(indices_current)
                                        );
                                        
                                        face_point[c] += 0.5 * grid.cell_length[c]; // positive-face convention
                                        let v = self.velocity_at_point(face_point);
                                        velocity[flat_index_current][c] = v[c];
                                    }
                                } else {
                                    velocity[flat_index_current][0] = velocity[flat_index_neighbor][0];
                                    velocity[flat_index_current][1] = velocity[flat_index_neighbor][1];
                                    velocity[flat_index_current][2] = velocity[flat_index_neighbor][2];
                                }
                            },
                            VelocityBoundaryCondition::ZeroGradient => {
                                velocity[flat_index_current][0] = velocity[flat_index_neighbor][0];
                                velocity[flat_index_current][1] = velocity[flat_index_neighbor][1];
                                velocity[flat_index_current][2] = velocity[flat_index_neighbor][2];
                            },
                            VelocityBoundaryCondition::SlipWall => {
                                velocity[flat_index_current][0] = velocity[flat_index_neighbor][0];
                                velocity[flat_index_current][1] = velocity[flat_index_neighbor][1];
                                velocity[flat_index_current][2] = velocity[flat_index_neighbor][2];

                                velocity[flat_index_current][axis_index] = 0.0;
                            }
                        }
                    }
                }
            }
        }
    }
}