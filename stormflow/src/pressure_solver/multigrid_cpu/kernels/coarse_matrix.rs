use stormath::matrix::Matrix;
use stormath::type_aliases::Float;

use crate::grid::Grid;
use crate::pressure_solver::boundary_conditions::PressureBoundaryConditions;

use super::jacobi::boundary_sign;

/// One axis' contribution to a matrix row for the interior cell at `position` (0-indexed from
/// this axis' `0`-face) out of `count` interior cells, `stride` apart. Mirrors
/// `jacobi::axis_off_diagonal_sum` term for term — including its boundary-folded substitutions and
/// its per-cell, per-offset second-derivative `weights` — so the assembled matrix represents
/// exactly the same equation the Jacobi smoother iterates towards, just solved directly. See that
/// function's doc comment for the boundary-folding rationale.
#[inline]
fn add_axis_off_diagonal_entries(
    matrix: &mut Matrix<Float>,
    idx: usize,
    stride: usize,
    position: usize,
    count: usize,
    sign_min: Float,
    sign_max: Float,
    weights: &[Float; 5],
) {
    let weight_m2 = weights[0];
    let weight_m1 = weights[1];
    let weight_p1 = weights[3];
    let weight_p2 = weights[4];

    if position >= 1 {
        matrix[[idx, idx - stride]] += weight_m1;
    } else {
        matrix[[idx, idx]] += weight_m1 * sign_min;
    }

    if position >= 2 {
        matrix[[idx, idx - 2 * stride]] += weight_m2;
    } else if position == 1 {
        matrix[[idx, idx - stride]] += weight_m2 * sign_min;
    } else {
        matrix[[idx, idx + stride]] += weight_m2 * sign_min;
    }

    if position + 1 < count {
        matrix[[idx, idx + stride]] += weight_p1;
    } else {
        matrix[[idx, idx]] += weight_p1 * sign_max;
    }

    if position + 2 < count {
        matrix[[idx, idx + 2 * stride]] += weight_p2;
    } else if position + 1 < count {
        matrix[[idx, idx + stride]] += weight_p2 * sign_max;
    } else {
        matrix[[idx, idx - stride]] += weight_p2 * sign_max;
    }
}

/// Assembles the dense matrix for the 4th order accurate Poisson stencil (the same equation
/// `jacobi::jacobi_kernel` iterates towards) on `grid`'s interior cells, with boundary conditions
/// folded into the stencil exactly as the smoother does.
///
/// Meant for the coarsest multigrid level, where the interior cell count is small enough for a
/// dense direct solve to be cheap: `MultigridCPU` builds this once and reuses it for every
/// V-cycle (see `MultigridCPU::coarse_matrix`), solving that level exactly instead of relying on
/// many extra Jacobi iterations to approximate the same equation.
pub fn build_poisson_matrix4(
    grid: &Grid,
    boundary_conditions: &PressureBoundaryConditions,
) -> Matrix<Float> {
    let n = grid.nr_interior_cells();
    let mut matrix: Matrix<Float> = Matrix::new_default([n, n]);

    let [nx, ny, nz] = grid.interior_shape;
    let [sx, sy, sz] = grid.interior_stride;

    for idx in 0..n {
        let indices = grid.interior_indices_from_flat_index(idx);
        let [ii, ji, ki] = indices;

        matrix[[idx, idx]] += grid.poisson_diagonal4(indices);

        add_axis_off_diagonal_entries(
            &mut matrix, idx, sx, ii, nx,
            boundary_sign(boundary_conditions, 0, 0), boundary_sign(boundary_conditions, 0, 1),
            grid.poisson_axis_stencil_interior(0, ii)
        );
        add_axis_off_diagonal_entries(
            &mut matrix, idx, sy, ji, ny,
            boundary_sign(boundary_conditions, 1, 0), boundary_sign(boundary_conditions, 1, 1),
            grid.poisson_axis_stencil_interior(1, ji)
        );
        add_axis_off_diagonal_entries(
            &mut matrix, idx, sz, ki, nz,
            boundary_sign(boundary_conditions, 2, 0), boundary_sign(boundary_conditions, 2, 1),
            grid.poisson_axis_stencil_interior(2, ki)
        );
    }

    matrix
}

#[cfg(test)]
mod tests {
    use super::*;

    use stormath::spatial_vector::SpatialVector;

    use crate::pressure_solver::boundary_conditions::PressureBoundaryCondition;
    use super::super::jacobi::off_diagonal_sum;

    /// The assembled matrix must represent exactly the equation `jacobi_kernel` iterates
    /// towards: for every row, `(A * x)[idx]` should equal `poisson_diagonal4 * x[idx] +
    /// off_diagonal_sum(...)`, including the boundary-folded substitutions near each face.
    #[test]
    fn matrix_matches_boundary_folded_stencil() {
        let grid = Grid::new(
            SpatialVector([0.0, 0.0, 0.0]),
            SpatialVector([1.0, 1.0, 1.0]),
            [4, 3, 5]
        );

        let boundary_conditions = PressureBoundaryConditions::new_custom([
            [PressureBoundaryCondition::ZeroGradient, PressureBoundaryCondition::ZeroValue],
            [PressureBoundaryCondition::ZeroValue, PressureBoundaryCondition::ZeroGradient],
            [PressureBoundaryCondition::ZeroGradient, PressureBoundaryCondition::ZeroGradient],
        ]);

        let matrix = build_poisson_matrix4(&grid, &boundary_conditions);

        let n = grid.nr_interior_cells();
        let x: Vec<Float> = (0..n).map(|i| ((i as Float) * 0.37).sin()).collect();

        let ax = matrix.vector_multiply(&x);

        for idx in 0..n {
            let indices = grid.interior_indices_from_flat_index(idx);
            let expected = grid.poisson_diagonal4(indices) * x[idx]
                + off_diagonal_sum(&grid, &boundary_conditions, &x, idx, indices);

            assert!(
                (ax[idx] - expected).abs() < 1e-4 * expected.abs().max(1.0),
                "mismatch at {idx}: matrix gives {}, stencil gives {}", ax[idx], expected
            );
        }
    }
}
