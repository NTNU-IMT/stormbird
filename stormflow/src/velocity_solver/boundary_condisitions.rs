
use stormath::spatial_vector::SpatialVector;

use stormbird::wind::{
    environment::WindEnvironment,
    wind_condition::WindCondition,
};

use crate::grid::Grid;
use crate::grid::INTERIOR_OFFSET;
use crate::grid::boundary_face::BoundaryFace;

use rayon::prelude::*;

#[derive(Debug, Clone, Copy)]
pub enum VelocityBoundaryCondition {
    ZeroGradient,
    InletOutlet,
    SlipWall,
}

impl VelocityBoundaryCondition {
    /// GPU-side flag matching the constants in the velocity solver's `ghost_cells.wgsl`
    pub fn as_gpu_flag(&self) -> u32 {
        match self {
            VelocityBoundaryCondition::ZeroGradient => 0,
            VelocityBoundaryCondition::InletOutlet => 1,
            VelocityBoundaryCondition::SlipWall => 2,
        }
    }
}

#[derive(Debug, Clone)]
/// Boundary conditions for the staggered velocity field.
///
/// The inlet velocity is steady and only varies with height, and the up-direction is aligned with
/// one of the grid axes (`up_axis`). It is therefore precomputed at construction as a 1D profile
/// over the extended cell layers along `up_axis`, so that no Stormbird functionality is needed
/// when the ghost cells are updated.
pub struct VelocityBoundaryConditions {
    pub face_conditions: [[VelocityBoundaryCondition; 2]; 3],
    /// The grid axis that is aligned with the up-direction of the wind environment
    pub up_axis: usize,
    /// Staggered inlet velocity for each extended cell layer along `up_axis`. Component `c` of
    /// entry `k` is the inflow velocity component `c` evaluated at the positive `c`-face of a cell
    /// with extended index `k` along `up_axis`. Length equals `grid.extended_shape[up_axis]`.
    pub inlet_velocity_profile: Vec<SpatialVector>,
}

impl VelocityBoundaryConditions {
    pub fn new(
        wind_environment: &WindEnvironment,
        wind_condition: &WindCondition,
        linear_velocity: SpatialVector,
        slip_wall_boundary_override: [[bool; 2]; 3],
        grid: &Grid,
    ) -> Self {
        let up_direction = wind_environment.up_direction;

        let mut up_axis: usize = 0;

        if up_direction[1].abs() > up_direction[0].abs()  &&
            up_direction[1].abs() > up_direction[2].abs() {
            up_axis = 1;
        } else if up_direction[2].abs() > up_direction[0].abs()  &&
            up_direction[2].abs() > up_direction[1].abs() {
            up_axis = 2;
        }

        // The precomputed profile is only exact if the height depends on the up-axis coordinate
        // alone, which requires the up-direction to be aligned with a grid axis.
        for axis_index in 0..3 {
            if axis_index != up_axis {
                assert!(
                    up_direction[axis_index].abs() < 1e-6 * up_direction[up_axis].abs(),
                    "The up direction must be aligned with one of the grid axes. Got {:?}",
                    up_direction
                );
            }
        }

        let mut face_conditions = [[VelocityBoundaryCondition::InletOutlet; 2]; 3];

        face_conditions[up_axis][0] = VelocityBoundaryCondition::SlipWall;
        face_conditions[up_axis][1] = VelocityBoundaryCondition::ZeroGradient;

        // A bit of a hacky way to apply override the BC to slip walls if the user has set to so
        for axis_index in 0..3 {
            for face_index in 0..2 {
                if slip_wall_boundary_override[axis_index][face_index] {
                    face_conditions[axis_index][face_index] = VelocityBoundaryCondition::SlipWall;
                }
            }
        }

        let inlet_velocity_profile = Self::compute_inlet_velocity_profile(
            wind_environment,
            wind_condition,
            linear_velocity,
            up_axis,
            grid
        );

        Self {
            face_conditions,
            up_axis,
            inlet_velocity_profile
        }
    }

    /// Evaluates the steady apparent wind at the staggered face positions of each extended cell
    /// layer along `up_axis`. Only the up-axis coordinate of the evaluation points matter, so the
    /// other coordinates are taken at the grid start point.
    fn compute_inlet_velocity_profile(
        wind_environment: &WindEnvironment,
        wind_condition: &WindCondition,
        linear_velocity: SpatialVector,
        up_axis: usize,
        grid: &Grid,
    ) -> Vec<SpatialVector> {
        (0..grid.extended_shape[up_axis]).map(|i_up| {
            let mut extended_indices = [0; 3];
            extended_indices[up_axis] = i_up;

            let cell_center = grid.cell_center_extended(extended_indices);

            let mut velocity = SpatialVector::default();

            for c in 0..3 {
                let mut face_point = cell_center;
                face_point[c] += 0.5 * grid.cell_length[c]; // positive-face convention

                velocity[c] = wind_environment.steady_apparent_wind_velocity_vector_at_location(
                    wind_condition, face_point, linear_velocity
                )[c];
            }

            velocity
        }).collect()
    }

    #[inline(always)]
    /// Returns the precomputed staggered inlet velocity for the cell at `flat_index` on the
    /// extended grid
    pub fn inlet_velocity(&self, grid: &Grid, flat_index: usize) -> SpatialVector {
        let i_up = (flat_index / grid.extended_stride[self.up_axis]) %
            grid.extended_shape[self.up_axis];

        self.inlet_velocity_profile[i_up]
    }

    pub fn initial_velocity(&self, grid: &Grid) -> Vec<SpatialVector> {
        (0..grid.nr_extended_cells()).into_par_iter().map(|i_flat_extended| {
            self.inlet_velocity(grid, i_flat_extended)
        }).collect()
    }

    /// Updates the ghost cells on one face (`axis_index`/`face_index`), in parallel over the
    /// face's cells. Mirrors `PressureBoundaryConditions::set_ghost_cells_kernel`: a raw pointer
    /// lets the closure write `flat_current` while reads of `flat_neighbor` go through the
    /// ordinary slice, which is sound because `flat_current`/`flat_neighbor` are always on
    /// different layers along `axis_index` and this kernel only ever runs for one face at a time.
    fn set_ghost_cells_kernel(
        &self,
        axis_index: usize,
        condition: VelocityBoundaryCondition,
        boundary_face: &BoundaryFace,
        grid: &Grid,
        velocity: &mut [SpatialVector],
    ) {
        let velocity_ptr = velocity.as_mut_ptr() as usize;
        let [outer_len, inner_len] = boundary_face.shape;

        (0..outer_len * inner_len)
            .into_par_iter()
            .with_min_len(2048)
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
                    VelocityBoundaryCondition::InletOutlet => {
                        // `neighbor_delta`is positive on the min-boundary face (neighbor is toward
                        // +axis) and negative on the max-boundary face, so its sign alone tells us
                        // which flow direction counts as inflow, without needing `face_index` here.
                        let at_the_min_boundary_face = boundary_face.neighbor_delta > 0;

                        let neighbor_axis_flow = velocity[flat_neighbor][axis_index];

                        let inflow = if at_the_min_boundary_face{
                            neighbor_axis_flow > 0.0
                        } else {
                            neighbor_axis_flow < 0.0
                        };

                        if inflow {
                            self.inlet_velocity(grid, flat_current)
                        } else {
                            velocity[flat_neighbor]
                        }
                    },
                    VelocityBoundaryCondition::ZeroGradient => velocity[flat_neighbor],
                    VelocityBoundaryCondition::SlipWall => {
                        let mut v = velocity[flat_neighbor];
                        v[axis_index] = 0.0;
                        v
                    }
                };

                unsafe {
                    *(velocity_ptr as *mut SpatialVector).add(flat_current) = new_value;
                }
            });
    }

    /// Updates the ghost cells on the velocity, using the boundary conditions in self and the
    /// supplied grid for the indexing logic.
    pub fn set_ghost_cells(&self, grid: &Grid, velocity: &mut [SpatialVector]) {
        debug_assert_eq!(
            self.inlet_velocity_profile.len(),
            grid.extended_shape[self.up_axis],
            "The inlet velocity profile does not match the grid"
        );

        for axis_index in 0..3 {
            for face_index in 0..2 {
                let condition = self.face_conditions[axis_index][face_index];

                for ghost_layer in 0..INTERIOR_OFFSET {
                    let boundary_face = BoundaryFace::new(
                        grid.extended_shape,
                        grid.extended_stride,
                        axis_index,
                        face_index,
                        ghost_layer
                    );

                    self.set_ghost_cells_kernel(axis_index, condition, &boundary_face, grid, velocity);
                }
            }
        }
    }
}
