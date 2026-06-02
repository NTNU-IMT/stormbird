use stormath::type_aliases::Float;

use crate::grid::Grid;

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

const PLANE_AXIS: [(usize, usize); 3] = [
    (1, 2),
    (0, 2),
    (0, 1)
];

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
        current_axis_offset: usize,
        inner_n: usize,
        outer_n: usize,
        inner_stride: usize,
        outer_stride: usize,
        neighbor_delta: isize,
        p: &mut [Float]
    ) {
        let p_ptr = p.as_mut_ptr() as usize;
        
        (0..outer_n)
            .into_par_iter()
            .with_min_len(2048)
            .for_each(|i_outer| {
                let row_base = current_axis_offset + i_outer * outer_stride;
                let mut flat_current = row_base;

                for _ in 0..inner_n {
                    let flat_neighbor = (flat_current as isize + neighbor_delta) as usize;

                    let new_value = match condition {
                        PressureBoundaryCondition::ZeroGradient => p[flat_neighbor],
                        PressureBoundaryCondition::ZeroValue => -p[flat_neighbor]
                    };

                    unsafe {
                        let ptr = p_ptr as *mut Float;

                        *ptr.add(flat_current) = new_value;
                    }

                    flat_current += inner_stride;
                }
            });
    }

    #[inline]
    /// Updates the ghost cells on the pressure, p, using the boundary conditions in self and the 
    /// supplied grid for the indexing logic
    pub fn set_ghost_cells(&self, grid: &Grid, p: &mut [Float]) {
        let shape = grid.extended_shape;
        let stride = grid.extended_stride;
    
        for axis_index in 0..3 {
            let axis_length = shape[axis_index];
            let axis_stride = stride[axis_index];
    
            // The two in-plane axes (everything that isn't axis_index).
            let (plane_axes_0, plane_axes_1) = PLANE_AXIS[axis_index];
    
            // Order the in-plane loops so the smaller stride is innermost
            // (best cache locality). With this layout that puts the
            // contiguous axis 2 innermost whenever it's in the plane.
            let (inner_axis, outer_axis) =
                if stride[plane_axes_0] <= stride[plane_axes_1] {
                    (plane_axes_0, plane_axes_1)
                } else {
                    (plane_axes_1, plane_axes_0)
                };
    
            let inner_n = shape[inner_axis];
            let outer_n = shape[outer_axis];
            let inner_stride = stride[inner_axis];
            let outer_stride = stride[outer_axis];
    
            for face_index in 0..2 {
                // Constant offsets for this layer, computed once per face.
                let (current_axis_offset, neighbor_delta) = if face_index == 0 {
                    // current layer 0, neighbor layer 1 -> neighbor = current + axis_stride
                    (0, axis_stride as isize)
                } else {
                    // current layer L-1, neighbor layer L-2 -> neighbor = current - axis_stride
                    ((axis_length - 1) * axis_stride, -(axis_stride as isize))
                };
    
                // Pick the operation once — it's constant across the whole face.
                let condition = self.face_conditions[axis_index][face_index];

                Self::set_ghost_cells_kernel(
                    &condition, 
                    current_axis_offset, 
                    inner_n, 
                    outer_n, 
                    inner_stride, 
                    outer_stride, 
                    neighbor_delta, 
                    p
                );
            }
        }
    }
}
