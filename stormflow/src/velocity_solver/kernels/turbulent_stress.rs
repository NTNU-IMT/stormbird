use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

use crate::grid::Grid;

use super::convect_and_diffuse::explicit_rate;

#[inline(always)]
/// The divergence of the turbulent stress, `d/dx_j [nu_t (du_i/dx_j + du_j/dx_i)]`, at the
/// staggered faces of cell `i_0`, discretized with the standard 2nd order MAC stencil: the normal
/// stresses live at the cell centers, where the eddy viscosity is stored, while the shear stresses
/// live at the cell edges, where the eddy viscosity is averaged from the four surrounding cells.
///
/// The `nu_t du_i/dx_j` part is split into a diagonal coefficient, which is returned separately so
/// that the caller can treat it implicitly, and the neighbor contributions. The `nu_t du_j/dx_i`
/// part (the transpose) is treated explicitly, except along the normal direction, where it is
/// identical to the first part. Returns `(explicit, diagonal)`, where the full term for component
/// `i` is `explicit[i] - diagonal[i] * velocity[i_0][i]`.
///
/// Treating the diagonal implicitly makes the diffusion unconditionally stable, as the
/// discretization is diagonally dominant, so a large eddy viscosity doesn't limit the time step.
/// This relies on the stencil being 2nd order: the 4th order Laplacian used for the molecular
/// viscosity is not diagonally dominant.
pub fn turbulent_stress_terms(
    i_0: usize,
    grid: &Grid,
    velocity: &[SpatialVector],
    eddy_viscosity: &[Float],
) -> (SpatialVector, SpatialVector) {
    let stride = grid.extended_stride;

    let mut explicit = SpatialVector::default();
    let mut diagonal = SpatialVector::default();

    for i in 0..3 {
        let s_i = stride[i];

        // The two cells on each side of the u_i-face
        let c_0 = i_0;
        let c_1 = i_0 + s_i;

        let nu_c_0 = eddy_viscosity[c_0];
        let nu_c_1 = eddy_viscosity[c_1];

        // Normal stress, where the transpose doubles the term
        let inv_h2_i = grid.inv_cell_length_squared[i];

        diagonal[i] += 2.0 * (nu_c_1 + nu_c_0) * inv_h2_i;
        explicit[i] += 2.0 * (
            nu_c_1 * velocity[i_0 + s_i][i] +
            nu_c_0 * velocity[i_0 - s_i][i]
        ) * inv_h2_i;

        // Shear stresses, at the edges above and below the face along j
        for j in 0..3 {
            if j == i {
                continue;
            }

            let s_j = stride[j];

            let nu_edge_p = 0.25 * (
                nu_c_0 + nu_c_1 + eddy_viscosity[c_0 + s_j] + eddy_viscosity[c_1 + s_j]
            );

            let nu_edge_m = 0.25 * (
                nu_c_0 + nu_c_1 + eddy_viscosity[c_0 - s_j] + eddy_viscosity[c_1 - s_j]
            );

            let inv_h2_j = grid.inv_cell_length_squared[j];

            diagonal[i] += (nu_edge_p + nu_edge_m) * inv_h2_j;
            explicit[i] += (
                nu_edge_p * velocity[i_0 + s_j][i] +
                nu_edge_m * velocity[i_0 - s_j][i]
            ) * inv_h2_j;

            // du_j/dx_i at the two edges. u_j lives on the j-faces, which are aligned with the
            // edges along i.
            let duj_dxi_p = (velocity[i_0 + s_i][j] - velocity[i_0][j]) * grid.inv_cell_length[i];
            let duj_dxi_m = (velocity[i_0 + s_i - s_j][j] - velocity[i_0 - s_j][j]) * grid.inv_cell_length[i];

            explicit[i] += (nu_edge_p * duj_dxi_p - nu_edge_m * duj_dxi_m) * grid.inv_cell_length[j];
        }
    }

    (explicit, diagonal)
}

#[inline(always)]
/// Same as `convect_and_diffuse_kernel`, but with the turbulent stresses from `eddy_viscosity`
/// added (see `turbulent_stress_terms`). The diagonal part of the turbulent stress is treated
/// implicitly, which makes each evaluation a Jacobi iteration on the implicit diffusion, where
/// the inner iterations of the time step act as the further Jacobi iterations.
pub fn convect_and_diffuse_turbulent_kernel(
    i_0: usize,
    grid: &Grid,
    velocity_org: &[SpatialVector],
    velocity: &[SpatialVector],
    body_force: &[SpatialVector],
    eddy_viscosity: &[Float],
    viscosity: Float,
    inv_density: Float,
    time_step: Float
) -> SpatialVector {
    let rate = explicit_rate(i_0, grid, velocity, body_force, viscosity, inv_density);

    let (explicit, diagonal) = turbulent_stress_terms(i_0, grid, velocity, eddy_viscosity);

    let mut out = SpatialVector::default();

    for i in 0..3 {
        out[i] = (velocity_org[i_0][i] + time_step * (rate[i] + explicit[i])) /
            (1.0 + time_step * diagonal[i]);
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_grid() -> Grid {
        Grid::new(SpatialVector([0.0; 3]), SpatialVector([1.0, 1.5, 2.0]), [8, 8, 8])
    }

    /// For a constant eddy viscosity and a divergence free field, the turbulent stress is
    /// `nu_t * laplacian(u)`, which is exact for the 2nd order stencil when the field is quadratic.
    #[test]
    fn constant_eddy_viscosity_gives_the_laplacian() {
        let grid = test_grid();
        let nu = 0.7;

        // u = y^2, v = z^2, w = x^2, which is divergence free with laplacian (2, 2, 2)
        let velocity: Vec<SpatialVector> = (0..grid.nr_extended_cells()).map(|i| {
            let center = grid.cell_center_extended(grid.extended_indices_from_flat_index(i));

            let mut v = SpatialVector::default();
            for component in 0..3 {
                let mut face = center;
                face[component] += 0.5 * grid.cell_length[component];

                v[component] = match component {
                    0 => face[1].powi(2),
                    1 => face[2].powi(2),
                    _ => face[0].powi(2),
                };
            }

            v
        }).collect();

        let eddy_viscosity = vec![nu; grid.nr_extended_cells()];

        let i_0 = grid.flat_index_on_extended_grid_from_interior_indices([4, 3, 5]);

        let (explicit, diagonal) = turbulent_stress_terms(i_0, &grid, &velocity, &eddy_viscosity);

        for i in 0..3 {
            let value = explicit[i] - diagonal[i] * velocity[i_0][i];

            assert!((value - 2.0 * nu).abs() < 1e-3, "component {i}: {value}");
        }
    }

    /// The transpose part only contributes when the eddy viscosity varies. For u = (y, 0, 0) and
    /// nu_t = x, the stress is d/dy(nu_t du/dy) + d/dx(nu_t dv/dx)... = 0 for u, while v gets
    /// d/dx(nu_t du/dy) = 1.
    #[test]
    fn transpose_part_with_varying_eddy_viscosity() {
        let grid = test_grid();

        let velocity: Vec<SpatialVector> = (0..grid.nr_extended_cells()).map(|i| {
            let center = grid.cell_center_extended(grid.extended_indices_from_flat_index(i));

            SpatialVector([center[1], 0.0, 0.0])
        }).collect();

        let eddy_viscosity: Vec<Float> = (0..grid.nr_extended_cells()).map(|i| {
            grid.cell_center_extended(grid.extended_indices_from_flat_index(i))[0]
        }).collect();

        let i_0 = grid.flat_index_on_extended_grid_from_interior_indices([4, 3, 5]);

        let (explicit, diagonal) = turbulent_stress_terms(i_0, &grid, &velocity, &eddy_viscosity);

        let values: Vec<Float> = (0..3).map(|i| explicit[i] - diagonal[i] * velocity[i_0][i]).collect();

        assert!(values[0].abs() < 1e-4, "{values:?}");
        assert!((values[1] - 1.0).abs() < 1e-4, "{values:?}");
        assert!(values[2].abs() < 1e-4, "{values:?}");
    }
}
