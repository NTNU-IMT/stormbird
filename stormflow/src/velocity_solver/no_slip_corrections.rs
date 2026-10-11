use std::collections::HashMap;

use stormath::type_aliases::Float;

use crate::grid::Grid;
use crate::geometry::Geometry;

#[derive(Debug, Clone, Copy)]
/// A precomputed no-slip correction for one staggered velocity face (one cell, one axis).
pub struct NoSlipEntry {
    /// Flat extended-grid index of the corrected face's base cell.
    pub cell_index: usize,
    /// Blend factor between the cell's own velocity and zero velocity: `0` inside the body, `1` in
    /// the fluid (such faces get no entry at all, as the correction leaves them unchanged).
    pub mu: Float,
}

#[derive(Debug, Clone, Default)]
/// Sparse list of the velocity faces affected by the no-slip geometries. The geometry is static, so
/// the blend factor of each face is computed once, and only faces with `mu < 1` are stored. This
/// avoids touching the full velocity field every time the correction is applied.
pub struct NoSlipCorrections {
    /// One entry list per corrected velocity component/axis.
    pub entries: [Vec<NoSlipEntry>; 3],
}

impl NoSlipCorrections {
    /// Builds the correction entries for every interior cell/axis where the blending function,
    /// evaluated with the signed distance at the staggered face, is below one.
    pub fn build(
        grid: &Grid,
        signed_distance_function: &[Float],
        epsilon: Float,
    ) -> Self {
        let mut entries: [Vec<NoSlipEntry>; 3] = Default::default();

        let [nxi, nyi, nzi] = grid.interior_shape;

        for ii in 0..nxi {
            for ji in 0..nyi {
                for ki in 0..nzi {
                    let extended_indices = grid.extended_indices_from_interior_indices([ii, ji, ki]);
                    let i_0 = grid.flat_index_on_extended_grid(extended_indices);

                    for axis_index in 0..3 {
                        let mut extended_indices_p = extended_indices;
                        extended_indices_p[axis_index] += 1;

                        let i_p = grid.flat_index_on_extended_grid(extended_indices_p);

                        let sdf = 0.5 * (
                            signed_distance_function[i_0] +
                            signed_distance_function[i_p]
                        );

                        let mu = Geometry::blending_function(sdf, epsilon);

                        if mu < 1.0 {
                            entries[axis_index].push(NoSlipEntry {
                                cell_index: i_0,
                                mu,
                            });
                        }
                    }
                }
            }
        }

        Self { entries }
    }
}

impl NoSlipCorrections {
    /// Combines two sets of corrections, using the smallest blend factor for the faces that are in
    /// both. Each face gets at most one entry, which the correction kernels rely on.
    pub fn merged(self, other: Self) -> Self {
        let mut entries: [Vec<NoSlipEntry>; 3] = Default::default();

        for (axis, (self_entries, other_entries)) in self.entries.into_iter().zip(other.entries).enumerate() {
            let mut mu_per_cell: HashMap<usize, Float> = HashMap::new();

            for entry in self_entries.into_iter().chain(other_entries) {
                mu_per_cell.entry(entry.cell_index)
                    .and_modify(|mu| *mu = mu.min(entry.mu))
                    .or_insert(entry.mu);
            }

            entries[axis] = mu_per_cell.into_iter()
                .map(|(cell_index, mu)| NoSlipEntry { cell_index, mu })
                .collect();

            entries[axis].sort_by_key(|entry| entry.cell_index);
        }

        Self { entries }
    }
}
