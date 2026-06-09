use stormath::type_aliases::Float;

use crate::grid::Grid;
use crate::grid::boundary_face::BoundaryFace;

use stormath::spatial_vector::SpatialVector;

use rayon::prelude::*;

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

    pub fn set_ghost_cells_kernel(
        condition: &PressureBoundaryCondition,
        boundary_face: &BoundaryFace,
        p: &mut [Float]
    ) {
        let p_ptr = p.as_mut_ptr() as usize;
        let [outer_len, inner_len] = boundary_face.shape;
    
        (0..outer_len * inner_len)
            .into_par_iter()
            .with_min_len(2048)
            .for_each(|idx| {
                let i_outer = idx / inner_len;
                let i_inner = idx % inner_len;
    
                let flat_current = (
                    boundary_face.axis_offset
                    + i_inner * boundary_face.stride[0]
                    + i_outer * boundary_face.stride[1]
                ) as usize;
    
                let flat_neighbor = (flat_current as i32 + boundary_face.neighbor_delta) as usize;
    
                let new_value = match condition {
                    PressureBoundaryCondition::ZeroGradient => p[flat_neighbor],
                    PressureBoundaryCondition::ZeroValue => -p[flat_neighbor],
                };
    
                unsafe {
                    *(p_ptr as *mut Float).add(flat_current) = new_value;
                }
            });
    }

    #[inline]
    /// Updates the ghost cells on the pressure, p, using the boundary conditions in self and the 
    /// supplied grid for the indexing logic
    pub fn set_ghost_cells(&self, grid: &Grid, p: &mut [Float]) {  
        for axis_index in 0..3 {
            for face_index in 0..2 {
                // Constant offsets for this layer, computed once per face.
                let boundary_face = BoundaryFace::new(
                    grid.extended_shape,
                    grid.extended_stride,
                    axis_index,
                    face_index
                );
    
                // Pick the operation once — it's constant across the whole face.
                let condition = self.face_conditions[axis_index][face_index];

                Self::set_ghost_cells_kernel(
                    &condition, 
                    &boundary_face, 
                    p
                );
            }
        }
    }
}
