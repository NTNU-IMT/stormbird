use stormath::type_aliases::Float;

use crate::grid::Grid;

/// Locates the fine cell center between two coarse cell centers along one axis.
///
/// Returns the lower coarse interior index and the interpolation weight of the *upper* one. Coarse
/// cell `i_f / 2` always contains fine cell `i_f` (coarsening merges cell pairs), so which side of
/// that coarse center the fine center falls on decides the bracketing pair — no search needed.
/// Fine cells outside the outermost pair of coarse centers are clamped, which degrades to constant
/// injection there just as the previous index-space version did.
#[inline(always)]
fn bracketing_coarse_cells(
    fine_grid: &Grid,
    coarse_grid: &Grid,
    axis: usize,
    fine_index: usize
) -> (usize, usize, Float) {
    let nr_coarse_cells = coarse_grid.interior_shape[axis];

    let coarse_index = (fine_index / 2).min(nr_coarse_cells - 1);

    let fine_center = fine_grid.cell_center_on_axis(axis, fine_index);
    let coarse_center = coarse_grid.cell_center_on_axis(axis, coarse_index);

    let (low, high) = if fine_center >= coarse_center {
        if coarse_index + 1 < nr_coarse_cells {
            (coarse_index, coarse_index + 1)
        } else {
            (coarse_index.saturating_sub(1), coarse_index)
        }
    } else if coarse_index >= 1 {
        (coarse_index - 1, coarse_index)
    } else {
        (coarse_index, (coarse_index + 1).min(nr_coarse_cells - 1))
    };

    if low == high {
        return (low, high, 0.0);
    }

    let center_low = coarse_grid.cell_center_on_axis(axis, low);
    let center_high = coarse_grid.cell_center_on_axis(axis, high);

    let weight = ((fine_center - center_low) / (center_high - center_low)).clamp(0.0, 1.0);

    (low, high, weight)
}

#[inline(always)]
/// Kernel for interpolating a solution from a coarse grid to a fine grid.
///
/// Trilinear interpolation in *physical* space: the weights come from where the fine cell center
/// actually sits between the two bracketing coarse cell centers. On a uniform grid the fine center
/// always sits a quarter of a coarse cell off its parent's center, which recovers the constant
/// `1/4`, `3/4` weights this used to hardcode; on a stretched grid it does not, and interpolating
/// in index space instead would place the correction in the wrong place and stop the V-cycle from
/// converging.
pub fn prolongate_and_correct_kernel(
    indices_fine: [usize; 3],
    fine_grid: &Grid,
    coarse_grid: &Grid,
    coarse_values: &[Float]
) -> Float {
    let mut low = [0usize; 3];
    let mut high = [0usize; 3];
    let mut weight_high = [0.0 as Float; 3];

    for axis in 0..3 {
        let (axis_low, axis_high, weight) = bracketing_coarse_cells(
            fine_grid, coarse_grid, axis, indices_fine[axis]
        );

        low[axis] = axis_low;
        high[axis] = axis_high;
        weight_high[axis] = weight;
    }

    let mut correction_value: Float = 0.0;

    for (di, wx) in [1.0 - weight_high[0], weight_high[0]].into_iter().enumerate() {
        for (dj, wy) in [1.0 - weight_high[1], weight_high[1]].into_iter().enumerate() {
            for (dk, wz) in [1.0 - weight_high[2], weight_high[2]].into_iter().enumerate() {
                let weight = wx * wy * wz;

                if weight == 0.0 {
                    continue;
                }

                let idx_coarse = coarse_grid.flat_index_on_interior_grid([
                    if di == 0 { low[0] } else { high[0] },
                    if dj == 0 { low[1] } else { high[1] },
                    if dk == 0 { low[2] } else { high[2] },
                ]);

                correction_value += weight * coarse_values[idx_coarse];
            }
        }
    }

    correction_value
}
