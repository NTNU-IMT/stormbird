use stormath::type_aliases::Float;

use crate::grid::Grid;
use crate::grid::INTERIOR_OFFSET;
use crate::grid::boundary_face::BoundaryFace;
use crate::velocity_solver::boundary_condisitions::{VelocityBoundaryConditions, VelocityBoundaryCondition};

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

impl PressureBoundaryCondition {
    /// GPU-side flag matching the `zero_value` uniform expected by `set_ghost_cells.wgsl`
    /// (0 = ZeroGradient, 1 = ZeroValue).
    pub fn as_gpu_flag(&self) -> u32 {
        match self {
            PressureBoundaryCondition::ZeroGradient => 0,
            PressureBoundaryCondition::ZeroValue => 1,
        }
    }
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

    /// Constructs the boundary conditions that are consistent with the velocity boundary
    /// conditions: the open top of the domain (zero gradient velocity) gets a zero pressure, while
    /// the slip walls and the inlet/outlet faces get a zero pressure gradient. This makes the
    /// pressure follow the ground/top placement of the velocity, including any slip wall
    /// overrides, so that no flow is driven through a slip wall by the pressure.
    ///
    /// # Panics
    /// Panics if no face gets a zero pressure, as the pressure equation then has no reference
    /// value. This happens if the open top of the domain is overridden to be a slip wall.
    pub fn new_from_velocity_boundary_conditions(
        velocity_boundary_conditions: &VelocityBoundaryConditions
    ) -> Self {
        let face_conditions = velocity_boundary_conditions.face_conditions.map(|axis_conditions| {
            axis_conditions.map(|condition| match condition {
                VelocityBoundaryCondition::ZeroGradient => PressureBoundaryCondition::ZeroValue,
                VelocityBoundaryCondition::InletOutlet => PressureBoundaryCondition::ZeroGradient,
                VelocityBoundaryCondition::SlipWall => PressureBoundaryCondition::ZeroGradient,
            })
        });

        let has_reference_pressure = face_conditions.iter().flatten().any(
            |condition| matches!(condition, PressureBoundaryCondition::ZeroValue)
        );

        assert!(
            has_reference_pressure,
            "At least one boundary must be open, with a zero pressure. The top of the domain is \
             the only open boundary, so it can not be overridden to be a slip wall."
        );

        Self {
            face_conditions
        }
    }

    /// Constructs boundary conditions directly from a full per-axis/per-face specification, for
    /// callers (e.g. tests, or solvers exercising boundary condition combinations that
    /// `new_from_up_direction` doesn't produce) that need explicit control over every face.
    pub fn new_custom(face_conditions: [[PressureBoundaryCondition; 2]; 3]) -> Self {
        Self {
            face_conditions
        }
    }

    /// Returns the boundary condition applied to the given axis/face combination.
    ///
    /// `axis_index` is 0/1/2 for x/y/z, `face_index` is 0 for the negative face and 1 for the
    /// positive face.
    pub fn condition(&self, axis_index: usize, face_index: usize) -> PressureBoundaryCondition {
        self.face_conditions[axis_index][face_index]
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
            .with_min_len(512)
            .for_each(|idx| {
                let i_outer = idx / inner_len;
                let i_inner = idx % inner_len;
    
                let flat_current = (
                    boundary_face.axis_offset
                    + i_outer * boundary_face.stride[0]
                    + i_inner * boundary_face.stride[1]
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
                // Pick the operation once — it's constant across the whole face.
                let condition = self.face_conditions[axis_index][face_index];

                for ghost_layer in 0..INTERIOR_OFFSET {
                    let boundary_face = BoundaryFace::new(
                        grid.extended_shape,
                        grid.extended_stride,
                        axis_index,
                        face_index,
                        ghost_layer
                    );

                    Self::set_ghost_cells_kernel(
                        &condition,
                        &boundary_face,
                        p
                    );
                }
            }
        }
    }
}
