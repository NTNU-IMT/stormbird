use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

use rayon::prelude::*;

use crate::grid::{Grid, INTERIOR_OFFSET};
use crate::grid::boundary_face::BoundaryFace;
use crate::velocity_solver::boundary_condisitions::{
    VelocityBoundaryConditions,
    VelocityBoundaryCondition
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Boundary condition for the cell-centered fields of the turbulence models
pub enum ScalarBoundaryCondition {
    ZeroGradient,
    /// The inlet value where the flow enters the domain, and zero gradient where it leaves it.
    InletOutlet,
}

impl ScalarBoundaryCondition {
    /// GPU-side flag matching the constants in the turbulence solver's `ghost_cells.wgsl`
    pub fn as_gpu_flag(&self) -> u32 {
        match self {
            Self::ZeroGradient => 0,
            Self::InletOutlet => 1,
        }
    }
}

#[derive(Debug, Clone)]
/// Boundary conditions for the transported fields of a turbulence model, which are stored
/// field-major in one array: value `f` of cell `i` is at index `f * nr_extended_cells + i`.
///
/// The conditions follow the velocity: the inlet/outlet faces of the velocity are inlet/outlet
/// faces for the turbulence fields, where the decision between inflow and outflow is made in the
/// same way as for the velocity, while the slip walls and the open top have zero gradient. The
/// inlet values only vary with height, and are precomputed as a profile along the up axis.
pub struct TurbulenceBoundaryConditions {
    pub face_conditions: [[ScalarBoundaryCondition; 2]; 3],
    /// The grid axis that is aligned with the up-direction of the wind environment
    pub up_axis: usize,
    pub nr_fields: usize,
    /// The inlet values for each extended cell layer along `up_axis`, layer-major: value `f` of
    /// layer `l` is at index `l * nr_fields + f`.
    pub inlet_profile: Vec<Float>,
}

impl TurbulenceBoundaryConditions {
    /// Uses the face conditions of the velocity. `inlet_values` gives the values of all the fields
    /// for each extended cell layer along the up axis.
    pub fn new(
        velocity_boundary_conditions: &VelocityBoundaryConditions,
        nr_fields: usize,
        inlet_values: impl Fn(usize) -> Vec<Float>,
        grid: &Grid
    ) -> Self {
        let face_conditions = velocity_boundary_conditions.face_conditions.map(|conditions| {
            conditions.map(|condition| match condition {
                VelocityBoundaryCondition::InletOutlet => ScalarBoundaryCondition::InletOutlet,
                VelocityBoundaryCondition::ZeroGradient |
                VelocityBoundaryCondition::SlipWall => ScalarBoundaryCondition::ZeroGradient,
            })
        });

        let up_axis = velocity_boundary_conditions.up_axis;

        let mut inlet_profile = Vec::with_capacity(grid.extended_shape[up_axis] * nr_fields);

        for layer in 0..grid.extended_shape[up_axis] {
            let values = inlet_values(layer);

            assert_eq!(values.len(), nr_fields);

            inlet_profile.extend_from_slice(&values);
        }

        Self {
            face_conditions,
            up_axis,
            nr_fields,
            inlet_profile,
        }
    }

    #[inline(always)]
    /// Returns the inlet value of `field_index` for the cell at `flat_index` on the extended grid
    pub fn inlet_value(&self, grid: &Grid, flat_index: usize, field_index: usize) -> Float {
        let layer = (flat_index / grid.extended_stride[self.up_axis]) %
            grid.extended_shape[self.up_axis];

        self.inlet_profile[layer * self.nr_fields + field_index]
    }

    /// The initial fields, which are the inlet values everywhere
    pub fn initial_fields(&self, grid: &Grid) -> Vec<Float> {
        let nr_extended_cells = grid.nr_extended_cells();

        let mut fields = vec![0.0; self.nr_fields * nr_extended_cells];

        for (field_index, field) in fields.chunks_exact_mut(nr_extended_cells).enumerate() {
            field.par_iter_mut().enumerate().for_each(|(flat_index, value)| {
                *value = self.inlet_value(grid, flat_index, field_index);
            });
        }

        fields
    }

    /// Updates the ghost cells of all the fields in `fields`. The inflow/outflow decision uses the
    /// velocity at the interior cell adjacent to the boundary, as in
    /// `VelocityBoundaryConditions::set_ghost_cells_kernel`.
    pub fn set_ghost_cells(&self, grid: &Grid, velocity: &[SpatialVector], fields: &mut [Float]) {
        set_ghost_cells(grid, &self.face_conditions, self.nr_fields, Some((self, velocity)), fields);
    }

    /// Sets zero gradient ghost cells on all faces of a single `field`, as used for the eddy
    /// viscosity.
    pub fn set_zero_gradient_ghost_cells(grid: &Grid, field: &mut [Float]) {
        set_ghost_cells(
            grid, &[[ScalarBoundaryCondition::ZeroGradient; 2]; 3], 1, None, field
        );
    }
}

/// Updates all ghost layers on all faces, one face at a time, in the same order as for the
/// velocity, since the edge and corner cells are written by more than one face. Each ghost layer
/// is paired with the interior cell that mirrors it across the boundary (see `BoundaryFace`).
/// `inlet` is only used by the inlet/outlet condition.
fn set_ghost_cells(
    grid: &Grid,
    face_conditions: &[[ScalarBoundaryCondition; 2]; 3],
    nr_fields: usize,
    inlet: Option<(&TurbulenceBoundaryConditions, &[SpatialVector])>,
    fields: &mut [Float],
) {
    let nr_extended_cells = grid.nr_extended_cells();

    assert_eq!(fields.len(), nr_fields * nr_extended_cells);

    let fields_ptr = fields.as_mut_ptr() as usize;

    for axis_index in 0..3 {
        for face_index in 0..2 {
            let condition = face_conditions[axis_index][face_index];

            let at_the_min_boundary_face = face_index == 0;

            let axis_stride = grid.extended_stride[axis_index] as isize;
            let inward = if at_the_min_boundary_face { axis_stride } else { -axis_stride };

            // All layers are filled by the same invocation, so that the inflow decision is made
            // once per column
            let boundary_faces: Vec<BoundaryFace> = (0..INTERIOR_OFFSET).map(|ghost_layer| {
                BoundaryFace::new(
                    grid.extended_shape, grid.extended_stride, axis_index, face_index, ghost_layer
                )
            }).collect();

            let [outer_len, inner_len] = boundary_faces[0].shape;

            (0..outer_len * inner_len)
                .into_par_iter()
                .with_min_len(2048)
                .for_each(|idx| {
                    let i_outer = idx / inner_len;
                    let i_inner = idx % inner_len;

                    for (ghost_layer, boundary_face) in boundary_faces.iter().enumerate() {
                        let flat_current = (
                            boundary_face.axis_offset
                            + i_outer * boundary_face.stride[0]
                            + i_inner * boundary_face.stride[1]
                        ) as usize;

                        let flat_neighbor = (flat_current as i32 + boundary_face.neighbor_delta) as usize;

                        let inflow = match (condition, inlet) {
                            (ScalarBoundaryCondition::InletOutlet, Some((_, velocity))) => {
                                let adjacent_distance = (ghost_layer + 1) as isize;
                                let flat_adjacent = (flat_current as isize + adjacent_distance * inward) as usize;

                                let adjacent_axis_flow = velocity[flat_adjacent][axis_index];

                                if at_the_min_boundary_face {
                                    adjacent_axis_flow > 0.0
                                } else {
                                    adjacent_axis_flow < 0.0
                                }
                            },
                            _ => false,
                        };

                        for field_index in 0..nr_fields {
                            let offset = field_index * nr_extended_cells;

                            let new_value = match (inflow, inlet) {
                                (true, Some((boundary_conditions, _))) => {
                                    boundary_conditions.inlet_value(grid, flat_current, field_index)
                                },
                                _ => {
                                    // Safe: the neighbor is an interior cell, which is never
                                    // written by this function
                                    unsafe { *(fields_ptr as *const Float).add(offset + flat_neighbor) }
                                }
                            };

                            // Safe: each invocation only writes the ghost cells in its own column
                            unsafe {
                                *(fields_ptr as *mut Float).add(offset + flat_current) = new_value;
                            }
                        }
                    }
                });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use stormbird::wind::{
        environment::WindEnvironment,
        wind_condition::WindCondition,
    };

    const N: usize = 8;

    fn test_grid() -> Grid {
        Grid::new(SpatialVector([0.0; 3]), SpatialVector([1.0; 3]), [N, N, N])
    }

    fn boundary_conditions(grid: &Grid) -> TurbulenceBoundaryConditions {
        let wind_environment = WindEnvironment {
            up_direction: SpatialVector([0.0, 0.0, 1.0]),
            wind_rotation_axis: SpatialVector([0.0, 0.0, 1.0]),
            ..Default::default()
        };

        let velocity_bcs = VelocityBoundaryConditions::new(
            &wind_environment,
            &WindCondition::new_constant(0.0, 10.0),
            SpatialVector::default(),
            [[false; 2]; 3],
            grid
        );

        TurbulenceBoundaryConditions::new(
            &velocity_bcs, 2, |layer| vec![100.0 + layer as Float, 200.0 + layer as Float], grid
        )
    }

    #[test]
    fn inflow_gets_inlet_values_and_outflow_zero_gradient() {
        let grid = test_grid();
        let bcs = boundary_conditions(&grid);

        let n = grid.nr_extended_cells();
        let cell = |i: usize| grid.flat_index_on_extended_grid([i, 5, 5]);
        let o = INTERIOR_OFFSET;

        for adjacent_flow in [1.0, -1.0] {
            let velocity = vec![SpatialVector([adjacent_flow, 0.0, 0.0]); n];
            let mut fields: Vec<Float> = (0..2 * n).map(|i| i as Float).collect();
            let original = fields.clone();

            bcs.set_ghost_cells(&grid, &velocity, &mut fields);

            for layer in 0..o {
                let ghost = cell(o - 1 - layer);

                for field_index in 0..2 {
                    let value = fields[field_index * n + ghost];

                    if adjacent_flow > 0.0 {
                        assert_eq!(value, bcs.inlet_value(&grid, ghost, field_index));
                    } else {
                        assert_eq!(value, original[field_index * n + cell(o + layer)]);
                    }
                }
            }
        }
    }
}
