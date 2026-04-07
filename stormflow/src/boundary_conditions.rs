
use stormath::type_aliases::Float;

use stormath::spatial_vector::SpatialVector;

use serde::{Serialize, Deserialize};

use super::grid::Grid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BoundaryConditionBuilder {
    Inlet(SpatialVector),
    Outlet,
    NoSlip,
    Slip,
}

#[derive(Debug, Clone)]
pub enum BoundaryCondition {
    Value(Float),
    ZeroGradient,
}

impl Default for BoundaryCondition {
    fn default() -> Self {
        Self::ZeroGradient
    }
}

#[derive(Debug, Clone, Default)]
pub struct BoundaryConditions {
    pub pressure: [[BoundaryCondition; 2]; 3],
    pub velocity_x: [[BoundaryCondition; 2]; 3],
    pub velocity_y: [[BoundaryCondition; 2]; 3],
    pub velocity_z: [[BoundaryCondition; 2]; 3],
}

impl BoundaryConditions {
    /// Updates the ghost cells on the velocity components, u, v, and w, using the boundary 
    /// conditions in self and the supplied grid for the indexing logic
    pub fn set_velocity_ghost_cells(
        &self, 
        grid: &Grid, 
        u: &mut [Float], 
        v: &mut [Float], 
        w: &mut [Float]
    ) {
        let [nx, ny, nz] = grid.nr_extended_cells.clone();
        
        // x-faces
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
                            
                        match self.velocity_x[axis_index][face_index] {
                            BoundaryCondition::Value(value) => {
                                if axis_index == 0 {
                                    // Principle: velocity_x is defined on the boundary face directly
                                    u[flat_index_current] = value
                                } else {
                                    // Principle: the x-component is half a cell length away from the boundary
                                    // face. u_face = 0.5 * (u_j + u_{j+1})
                                    // u_{j} = 2 * u_face - u_{j+1}
                                    u[flat_index_current] = 2.0 * value - u[flat_index_neighbor]
                                }
                                
                            },
                            BoundaryCondition::ZeroGradient => {
                                u[flat_index_current] = u[flat_index_neighbor]
                            }
                        }
                        
                        match self.velocity_y[axis_index][face_index] {
                            BoundaryCondition::Value(value) => {
                                if axis_index == 1 {
                                    v[flat_index_current] = value
                                } else {
                                    v[flat_index_current] = 2.0 * value - v[flat_index_neighbor]
                                }
                                
                            },
                            BoundaryCondition::ZeroGradient => {
                                v[flat_index_current] = v[flat_index_neighbor]
                            }
                        }
                        
                        match self.velocity_z[axis_index][face_index] {
                            BoundaryCondition::Value(value) => {
                                if axis_index == 2 {
                                    w[flat_index_current] = value
                                } else {
                                    w[flat_index_current] = 2.0 * value - w[flat_index_neighbor]
                                }
                            },
                            BoundaryCondition::ZeroGradient => {
                                w[flat_index_current] = w[flat_index_neighbor]
                            }
                        }
                    }
                }
            }
        }
    }
    
    /// Updates the ghost cells on the pressure, p, using the boundary conditions in self and the 
    /// supplied grid for the indexing logic
    pub fn set_pressure_ghost_cells(
        &self, 
        grid: &Grid, 
        p: &mut [Float]
    ) {
        let [nx, ny, nz] = grid.nr_extended_cells.clone();
        
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
                            
                        match self.pressure[axis_index][face_index] {
                            BoundaryCondition::Value(value) => {
                                p[flat_index_current] = 2.0 * value - p[flat_index_neighbor]
                            },
                            BoundaryCondition::ZeroGradient => {
                                p[flat_index_current] = p[flat_index_neighbor]
                            }
                        }
                    }
                }
            }
        }
    }
}