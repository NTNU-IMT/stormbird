
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

        // The ground is at the start of the up axis if the up direction points along the positive
        // axis, and at the end of it otherwise. The opposite face is the open top of the domain.
        let (ground_face_index, top_face_index) = if up_direction[up_axis] > 0.0 {
            (0, 1)
        } else {
            (1, 0)
        };

        face_conditions[up_axis][ground_face_index] = VelocityBoundaryCondition::SlipWall;
        face_conditions[up_axis][top_face_index] = VelocityBoundaryCondition::ZeroGradient;

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

    /// Updates one ghost layer (`ghost_layer`) on one face (`axis_index`/`face_index`), in parallel
    /// over the face's cells. Mirrors `PressureBoundaryConditions::set_ghost_cells_kernel`: a raw
    /// pointer lets the closure write `flat_current` while reads of interior cells go through the
    /// ordinary slice, which is sound because each invocation only reads and writes cells in its 
    /// own column through the face, and never reads a cell it writes.
    ///
    /// The ghost value is taken from the cell that mirrors the ghost cell across the boundary 
    /// (`boundary_face.neighbor_delta`), which is the right mirror for all velocity components 
    /// that are cell-centered along `axis_index`. The normal component lives on the faces along
    /// `axis_index`, so its mirror across the boundary is half a cell shifted. This only matters 
    /// for the slip walls, which mirror the normal component with the opposite sign: 
    ///
    /// - At the start of the axis, the normal face of ghost layer `0` lies on the wall, and is set
    ///   to zero, while ghost layer `l > 0` mirrors the interior face `2 * l` cells further in.
    /// - At the end of the axis, the face on the wall belongs to the last interior cell, and is 
    ///   set to zero together with ghost layer `0`, while ghost layer `l` mirrors the interior 
    ///   face `2 * l + 2` cells further in.
    ///
    /// For the inlet/outlet condition, the direction of the flow is checked at the interior cell
    /// adjacent to the boundary for all ghost layers, so that all layers in a column always use 
    /// the same condition.
    fn set_ghost_cells_kernel(
        &self,
        axis_index: usize,
        condition: VelocityBoundaryCondition,
        boundary_face: &BoundaryFace,
        ghost_layer: usize,
        grid: &Grid,
        velocity: &mut [SpatialVector],
    ) {
        let velocity_ptr = velocity.as_mut_ptr() as usize;
        let [outer_len, inner_len] = boundary_face.shape;

        // `neighbor_delta` is positive on the min-boundary face (neighbor is toward +axis) and
        // negative on the max-boundary face, so its sign alone tells which face this is.
        let at_the_min_boundary_face = boundary_face.neighbor_delta > 0;

        let axis_stride = grid.extended_stride[axis_index] as isize;
        let inward = if at_the_min_boundary_face { axis_stride } else { -axis_stride };

        // Distance, in cells, from the ghost cell to the interior cell adjacent to the boundary
        let adjacent_distance = (ghost_layer + 1) as isize;

        // Distance, in cells, from the ghost cell to the interior face that mirrors its normal
        // face across the wall. `None` means that the normal face is on the wall itself.
        let normal_mirror_distance = if at_the_min_boundary_face {
            if ghost_layer == 0 { None } else { Some(2 * ghost_layer as isize) }
        } else {
            Some(2 * ghost_layer as isize + 2)
        };

        let zero_wall_face_in_interior = !at_the_min_boundary_face && ghost_layer == 0;

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
                        let flat_adjacent = (flat_current as isize + adjacent_distance * inward) as usize;

                        let adjacent_axis_flow = velocity[flat_adjacent][axis_index];

                        let inflow = if at_the_min_boundary_face {
                            adjacent_axis_flow > 0.0
                        } else {
                            adjacent_axis_flow < 0.0
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

                        v[axis_index] = match normal_mirror_distance {
                            Some(distance) => {
                                let flat_mirror = (flat_current as isize + distance * inward) as usize;

                                -velocity[flat_mirror][axis_index]
                            },
                            None => 0.0
                        };

                        v
                    }
                };

                unsafe {
                    *(velocity_ptr as *mut SpatialVector).add(flat_current) = new_value;
                }

                if zero_wall_face_in_interior && matches!(condition, VelocityBoundaryCondition::SlipWall) {
                    let flat_wall = (flat_current as isize + inward) as usize;

                    unsafe {
                        let wall_cell = &mut *(velocity_ptr as *mut SpatialVector).add(flat_wall);
                        wall_cell[axis_index] = 0.0;
                    }
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

                    self.set_ghost_cells_kernel(
                        axis_index, condition, &boundary_face, ghost_layer, grid, velocity
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use stormath::type_aliases::Float;

    use crate::pressure_solver::boundary_conditions::{
        PressureBoundaryConditions, PressureBoundaryCondition
    };

    const N: usize = 8;

    fn test_grid() -> Grid {
        Grid::new(SpatialVector([0.0; 3]), SpatialVector([1.0; 3]), [N, N, N])
    }

    fn boundary_conditions(
        up_direction: SpatialVector,
        slip_wall_boundary_override: [[bool; 2]; 3],
        grid: &Grid
    ) -> VelocityBoundaryConditions {
        let wind_environment = WindEnvironment {
            up_direction,
            wind_rotation_axis: up_direction,
            ..Default::default()
        };

        VelocityBoundaryConditions::new(
            &wind_environment,
            &WindCondition::new_constant(0.0, 10.0),
            SpatialVector::default(),
            slip_wall_boundary_override,
            grid
        )
    }

    /// A velocity field where every component of every cell has a unique value
    fn unique_velocity(grid: &Grid) -> Vec<SpatialVector> {
        (0..grid.nr_extended_cells()).map(|i| {
            let value = i as Float;
            SpatialVector([value + 0.1, -value - 0.2, value + 0.3])
        }).collect()
    }

    #[test]
    fn ground_follows_the_sign_of_the_up_direction() {
        let grid = test_grid();

        for (up_direction, ground_face, top_face) in [
            (SpatialVector([0.0, 0.0, 1.0]), 0, 1),
            (SpatialVector([0.0, 0.0, -1.0]), 1, 0),
        ] {
            let velocity_bcs = boundary_conditions(up_direction, [[false; 2]; 3], &grid);
            let pressure_bcs = PressureBoundaryConditions::new_from_velocity_boundary_conditions(&velocity_bcs);

            assert!(matches!(velocity_bcs.face_conditions[2][ground_face], VelocityBoundaryCondition::SlipWall));
            assert!(matches!(velocity_bcs.face_conditions[2][top_face], VelocityBoundaryCondition::ZeroGradient));

            assert!(matches!(pressure_bcs.condition(2, ground_face), PressureBoundaryCondition::ZeroGradient));
            assert!(matches!(pressure_bcs.condition(2, top_face), PressureBoundaryCondition::ZeroValue));
        }
    }

    #[test]
    #[should_panic]
    fn overriding_the_top_to_a_slip_wall_is_rejected() {
        let grid = test_grid();
        let velocity_bcs = boundary_conditions(
            SpatialVector([0.0, 0.0, 1.0]), [[false; 2], [false; 2], [false, true]], &grid
        );

        PressureBoundaryConditions::new_from_velocity_boundary_conditions(&velocity_bcs);
    }

    /// Slip walls on both x faces: the normal (x) component is zero on the wall and mirrored with
    /// the opposite sign across it, while the tangential components are mirrored with the same sign.
    #[test]
    fn slip_walls_are_symmetry_planes_on_both_faces() {
        let grid = test_grid();
        let bcs = boundary_conditions(SpatialVector([0.0, 0.0, 1.0]), [[true, true], [false; 2], [false; 2]], &grid);

        let mut velocity = unique_velocity(&grid);
        let original = velocity.clone();

        bcs.set_ghost_cells(&grid, &mut velocity);

        // A column away from the other faces, so only the x faces affect it
        let cell = |i: usize| grid.flat_index_on_extended_grid([i, 5, 5]);

        let o = INTERIOR_OFFSET;
        let last_interior = N + o - 1;

        // Start of the axis: the face of ghost cell `o - 1` is on the wall
        assert_eq!(velocity[cell(o - 1)][0], 0.0);
        for layer in 1..o {
            assert_eq!(velocity[cell(o - 1 - layer)][0], -original[cell(o - 1 + layer)][0]);
        }

        // End of the axis: the face on the wall belongs to the last interior cell
        assert_eq!(velocity[cell(last_interior)][0], 0.0);
        for layer in 0..o {
            assert_eq!(velocity[cell(last_interior + 1 + layer)][0], -original[cell(last_interior - 1 - layer)][0]);
        }

        // Tangential components, which are cell-centered along x
        for layer in 0..o {
            for component in 1..3 {
                assert_eq!(velocity[cell(o - 1 - layer)][component], original[cell(o + layer)][component]);
                assert_eq!(velocity[cell(last_interior + 1 + layer)][component], original[cell(last_interior - layer)][component]);
            }
        }

        // Only the normal component of the wall face in the interior is changed
        for component in 1..3 {
            assert_eq!(velocity[cell(last_interior)][component], original[cell(last_interior)][component]);
        }
    }

    /// The inflow/outflow decision is made from the cell adjacent to the boundary, for all layers
    #[test]
    fn inlet_outlet_uses_one_decision_per_column() {
        let grid = test_grid();
        let bcs = boundary_conditions(SpatialVector([0.0, 0.0, 1.0]), [[false; 2]; 3], &grid);

        let cell = |i: usize| grid.flat_index_on_extended_grid([i, 5, 5]);
        let o = INTERIOR_OFFSET;

        for adjacent_flow in [1.0, -1.0] {
            let mut velocity = unique_velocity(&grid);

            // Flow direction at the adjacent cell, and the opposite further in
            for i in o..o + 2 * o {
                velocity[cell(i)][0] = -adjacent_flow;
            }
            velocity[cell(o)][0] = adjacent_flow;

            let original = velocity.clone();

            bcs.set_ghost_cells(&grid, &mut velocity);

            for layer in 0..o {
                let ghost = cell(o - 1 - layer);

                if adjacent_flow > 0.0 {
                    assert_eq!(velocity[ghost], bcs.inlet_velocity(&grid, ghost));
                } else {
                    assert_eq!(velocity[ghost], original[cell(o + layer)]);
                }
            }
        }
    }
}
