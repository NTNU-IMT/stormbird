use stormath::type_aliases::Float;

use crate::grid::Grid;

#[inline(always)]
/// Kernel for interpolating a solution from a coarse grid to a fine grid
pub fn prolongate_and_correct_kernel(
    indices_fine: [usize; 3],
    coarse_grid: &Grid,
    coarse_values: &[Float]
) -> Float {
    let [nx_c, ny_c, nz_c] = coarse_grid.interior_shape;
    
    // Fine cell center position in "coarse cell units"
    // Fine cell i_f has center at (i_f + 0.5) * dx_f = (i_f + 0.5) * dx_c / 2
    // In coarse cell units (where coarse cell j has center at j + 0.5),
    // the fine cell center is at: (i_f + 0.5) / 2 = i_f/2 + 0.25
    // We want position relative to coarse cell centers at j + 0.5,
    // so xi = (i_f + 0.5) / 2 - 0.5 = i_f/2 - 0.25
    let xi = (indices_fine[0] as Float) * 0.5 - 0.25;
    let eta = (indices_fine[1] as Float) * 0.5 - 0.25;
    let zeta = (indices_fine[2] as Float) * 0.5 - 0.25;

    // Find the "lower" coarse cell index for interpolation
    let i_c_base = (xi.floor() as isize).max(0).min((nx_c - 1) as isize) as usize;
    let j_c_base = (eta.floor() as isize).max(0).min((ny_c - 1) as isize) as usize;
    let k_c_base = (zeta.floor() as isize).max(0).min((nz_c - 1) as isize) as usize;

    // Local coordinates within the interpolation stencil [0, 1]
    let sx = (xi - (i_c_base as Float)).clamp(0.0, 1.0);
    let sy = (eta - (j_c_base as Float)).clamp(0.0, 1.0);
    let sz = (zeta - (k_c_base as Float)).clamp(0.0, 1.0);

    // Trilinear interpolation weights
    let wx = [1.0 - sx, sx];
    let wy = [1.0 - sy, sy];
    let wz = [1.0 - sz, sz];

    let mut correction_value: Float = 0.0;

    for di in 0..2 {
        for dj in 0..2 {
            for dk in 0..2 {
                let i_c = (i_c_base + di).min(nx_c - 1);
                let j_c = (j_c_base + dj).min(ny_c - 1);
                let k_c = (k_c_base + dk).min(nz_c - 1);

                let weight = wx[di] * wy[dj] * wz[dk];

                let idx_coarse = coarse_grid.flat_index_on_interior_grid([i_c, j_c, k_c]);

                correction_value += weight * coarse_values[idx_coarse];
            }
        }
    }

    correction_value
}