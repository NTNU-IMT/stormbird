use stormath::type_aliases::Float;

use crate::grid::Grid;

use stormath::spatial_vector::SpatialVector;


#[derive(Debug, Clone, Copy)]
/// The different boundary conditions that can be applied to the pressure field.
pub enum PressureBoundaryCondition {
    /// The ghost cells are set such that the pressure value at the boundary equals zero
    ZeroValue,
    /// The ghost cells are set such that the pressure gradient at the boundary equals zero
    ZeroGradient,
}

#[derive(Debug, Clone)]
pub struct PressureBoundaryConditions {
    face_conditions: [[PressureBoundaryCondition; 2]; 3]
}

impl PressureBoundaryConditions {
    pub fn new_from_up_direction(up_direction: SpatialVector) -> Self {
        let mut face_conditions = [[PressureBoundaryCondition::ZeroGradient; 2]; 3];

        let mut up_axis: usize = 0;

        if up_direction[1].abs() > up_direction[0].abs()  && up_direction[1].abs() > up_direction[2].abs() {
            up_axis = 1;
        } else if up_direction[2].abs() > up_direction[0].abs()  && up_direction[2].abs() > up_direction[1].abs() {
            up_axis = 2;
        }

        if up_direction[up_axis] > 0.0 {
            face_conditions[up_axis][1] = PressureBoundaryCondition::ZeroValue;
        } else {
            face_conditions[up_axis][0] = PressureBoundaryCondition::ZeroValue;
        }
        
        Self {
            face_conditions
        }
    }

    /// Updates the ghost cells on the pressure, p, using the boundary conditions in self and the 
    /// supplied grid for the indexing logic
    pub fn set_ghost_cells(
        &self, 
        grid: &Grid, 
        p: &mut [Float]
    ) {
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
                            PressureBoundaryCondition::ZeroValue => {
                                p[flat_index_current] = -p[flat_index_neighbor]
                            },
                            PressureBoundaryCondition::ZeroGradient => {
                                p[flat_index_current] = p[flat_index_neighbor]
                            }
                        }
                    }
                }
            }
        }
    }
}

impl PressureBoundaryCondition {
    
}