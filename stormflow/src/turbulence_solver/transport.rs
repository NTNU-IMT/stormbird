//! Model independent building blocks for the transport equations of the turbulence models, on
//! the CPU. The GPU versions are found in gpu/shaders/transport.wgsl.

use serde::{Serialize, Deserialize};

use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

use crate::grid::Grid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
/// The discretization of the convection term in the transport equations
pub enum ConvectionScheme {
    /// 1st order upwind. Bounded and very robust, but diffusive.
    Upwind,
    /// 2nd order, TVD limited, linear interpolation, applied as a deferred correction to the
    /// upwind scheme. The limiter is `psi(r) = max(0, min(2 r, 1))`, which is the same as the one
    /// used by OpenFOAM's `limitedLinear 1` scheme. Default.
    #[default]
    LimitedLinear,
}

impl ConvectionScheme {
    /// The flag used for the scheme in the shaders
    pub fn as_gpu_flag(&self) -> u32 {
        match self {
            Self::Upwind => 0,
            Self::LimitedLinear => 1,
        }
    }
}

#[inline(always)]
/// The velocity gradient tensor at the center of cell `i_0`, from the staggered velocity, where
/// `gradient[i][j] = du_i/dx_j`. The diagonal is the exact 2nd order difference across the cell,
/// while the off-diagonal terms are central differences of the cell-centered velocity in the
/// neighbor cells.
pub fn velocity_gradient(i_0: usize, grid: &Grid, velocity: &[SpatialVector]) -> [[Float; 3]; 3] {
    let stride = grid.extended_stride;

    let mut gradient = [[0.0; 3]; 3];

    for i in 0..3 {
        let s_i = stride[i];

        for j in 0..3 {
            gradient[i][j] = if i == j {
                (velocity[i_0][i] - velocity[i_0 - s_i][i]) * grid.inv_cell_length[i]
            } else {
                let s_j = stride[j];

                let u_p = velocity[i_0 + s_j][i] + velocity[i_0 + s_j - s_i][i];
                let u_m = velocity[i_0 - s_j][i] + velocity[i_0 - s_j - s_i][i];

                0.25 * (u_p - u_m) * grid.inv_cell_length[j]
            };
        }
    }

    gradient
}

#[inline(always)]
/// The limited deferred correction of the face value, relative to the upwind value, for a face
/// with the far upwind, upwind and downwind values `f_uu`, `f_u` and `f_d`.
fn limited_linear_correction(f_uu: Float, f_u: Float, f_d: Float) -> Float {
    let delta = f_d - f_u;

    if delta.abs() < 1e-30 {
        return 0.0;
    }

    let r = (f_u - f_uu) / delta;

    0.5 * (2.0 * r).clamp(0.0, 1.0) * delta
}

#[derive(Debug, Clone, Copy, Default)]
/// The discretized convection and diffusion of a transported field in one cell, split into the
/// coefficient of the cell's own value, and the sum of everything else.
pub struct ConvectionDiffusion {
    /// Coefficient of the cell's own value. Always non-negative.
    pub diagonal: Float,
    /// The contributions from the neighbor values, including the explicit deferred correction of
    /// the convection.
    pub neighbors: Float,
}

#[inline(always)]
/// Finite volume discretization of `div(u f) - f div(u) - div((viscosity + nu_t / sigma) grad f)`
/// for cell `i_0` of the cell-centered `field`, using the face velocities directly as the fluxes.
/// The `- f div(u)` term makes the convection bounded also when the velocity field is not exactly
/// divergence free, in the same way as OpenFOAM's `bounded` schemes. The upwind part of the
/// convection and the diffusion are returned as a diagonal coefficient and neighbor contributions,
/// so that the caller can treat them implicitly with a Jacobi iteration. The diffusion coefficient
/// on the faces uses the average of the eddy viscosity in the two cells.
///
/// The result is diagonally dominant, so the Jacobi iterations converge, and keeps the field
/// positive with the upwind scheme. The limited linear correction is lagged, i.e., evaluated from
/// `field` and added to the neighbor contributions.
pub fn convection_diffusion(
    i_0: usize,
    grid: &Grid,
    velocity: &[SpatialVector],
    eddy_viscosity: &[Float],
    field: &[Float],
    viscosity: Float,
    inv_sigma: Float,
    scheme: ConvectionScheme
) -> ConvectionDiffusion {
    let mut out = ConvectionDiffusion::default();

    let f_0 = field[i_0];
    let nu_t_0 = eddy_viscosity[i_0];

    for axis in 0..3 {
        let s = grid.extended_stride[axis];
        let inv_h = grid.inv_cell_length[axis];
        let inv_h2 = grid.inv_cell_length_squared[axis];

        // The outward flux through the positive and the negative face, together with the index
        // of the neighbor cell, the cell beyond the neighbor and the cell on the opposite side.
        let faces = [
            (velocity[i_0][axis], i_0 + s, i_0 + 2 * s, i_0 - s),
            (-velocity[i_0 - s][axis], i_0 - s, i_0 - 2 * s, i_0 + s),
        ];

        for (outward_flux, i_n, i_nn, i_opposite) in faces {
            let f_n = field[i_n];

            // Upwind convection, which is only non-zero for inflow faces
            let inflow = (-outward_flux).max(0.0) * inv_h;

            // Diffusion
            let diffusivity = viscosity + 0.5 * (nu_t_0 + eddy_viscosity[i_n]) * inv_sigma;
            let diffusion = diffusivity * inv_h2;

            out.diagonal += inflow + diffusion;
            out.neighbors += (inflow + diffusion) * f_n;

            if let ConvectionScheme::LimitedLinear = scheme {
                let correction = if outward_flux >= 0.0 {
                    limited_linear_correction(field[i_opposite], f_0, f_n)
                } else {
                    limited_linear_correction(field[i_nn], f_n, f_0)
                };

                out.neighbors -= outward_flux * correction * inv_h;
            }
        }
    }

    out
}

/// A slice that can be written to from several threads at once, for kernels where each cell
/// writes to its own indices in several output fields. The caller is responsible for never writing
/// to the same index from more than one thread.
pub(crate) struct ParallelWriteSlice<'a> {
    pointer: *mut Float,
    length: usize,
    _marker: std::marker::PhantomData<&'a mut [Float]>,
}

unsafe impl Send for ParallelWriteSlice<'_> {}
unsafe impl Sync for ParallelWriteSlice<'_> {}

impl<'a> ParallelWriteSlice<'a> {
    pub fn new(slice: &'a mut [Float]) -> Self {
        Self {
            pointer: slice.as_mut_ptr(),
            length: slice.len(),
            _marker: std::marker::PhantomData,
        }
    }

    #[inline(always)]
    /// # Safety
    /// No other thread may access `index` at the same time.
    pub unsafe fn write(&self, index: usize, value: Float) {
        assert!(index < self.length);

        unsafe { *self.pointer.add(index) = value; }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_grid() -> Grid {
        Grid::new(SpatialVector([0.0; 3]), SpatialVector([1.0, 1.5, 2.0]), [8, 8, 8])
    }

    fn staggered_field(grid: &Grid, f: impl Fn(SpatialVector) -> SpatialVector) -> Vec<SpatialVector> {
        (0..grid.nr_extended_cells()).map(|i| {
            let center = grid.cell_center_extended(grid.extended_indices_from_flat_index(i));

            let mut v = SpatialVector::default();
            for component in 0..3 {
                let mut face = center;
                face[component] += 0.5 * grid.cell_length[component];

                v[component] = f(face)[component];
            }

            v
        }).collect()
    }

    #[test]
    fn velocity_gradient_is_exact_for_linear_fields() {
        let grid = test_grid();

        let a = [[0.1, 0.2, 0.3], [-0.4, 0.5, 0.6], [0.7, -0.8, -0.6]];

        let velocity = staggered_field(&grid, |p| SpatialVector([
            a[0][0] * p[0] + a[0][1] * p[1] + a[0][2] * p[2],
            a[1][0] * p[0] + a[1][1] * p[1] + a[1][2] * p[2],
            a[2][0] * p[0] + a[2][1] * p[1] + a[2][2] * p[2],
        ]));

        let i_0 = grid.flat_index_on_extended_grid_from_interior_indices([3, 4, 5]);

        let gradient = velocity_gradient(i_0, &grid, &velocity);

        for i in 0..3 {
            for j in 0..3 {
                assert!((gradient[i][j] - a[i][j]).abs() < 1e-4, "{i}, {j}: {}", gradient[i][j]);
            }
        }
    }

    /// A constant field is unchanged by convection and diffusion, in any velocity field
    #[test]
    fn constant_field_has_no_convection_or_diffusion() {
        let grid = test_grid();

        let velocity = staggered_field(&grid, |p| SpatialVector([p[1], -p[0] * p[2], 0.3]));
        let eddy_viscosity: Vec<Float> = (0..grid.nr_extended_cells()).map(|i| 0.1 * i as Float).collect();
        let field = vec![2.5; grid.nr_extended_cells()];

        let i_0 = grid.flat_index_on_extended_grid_from_interior_indices([3, 4, 5]);

        for scheme in [ConvectionScheme::Upwind, ConvectionScheme::LimitedLinear] {
            let result = convection_diffusion(
                i_0, &grid, &velocity, &eddy_viscosity, &field, 0.01, 1.0 / 1.3, scheme
            );

            assert!(result.diagonal > 0.0);
            assert!((result.neighbors - result.diagonal * field[i_0]).abs() < 1e-3 * result.diagonal);
        }
    }

    /// The limited linear scheme is exact for a linear field in a uniform flow
    #[test]
    fn limited_linear_convection_of_linear_field_is_exact() {
        let grid = test_grid();

        let u = 2.0;
        let velocity = staggered_field(&grid, |_| SpatialVector([u, 0.0, 0.0]));
        let eddy_viscosity = vec![0.0; grid.nr_extended_cells()];

        let field: Vec<Float> = (0..grid.nr_extended_cells()).map(|i| {
            grid.cell_center_extended(grid.extended_indices_from_flat_index(i))[0]
        }).collect();

        let i_0 = grid.flat_index_on_extended_grid_from_interior_indices([3, 4, 5]);

        let result = convection_diffusion(
            i_0, &grid, &velocity, &eddy_viscosity, &field, 0.0, 1.0, ConvectionScheme::LimitedLinear
        );

        // The discretized operator is diagonal * f_0 - neighbors = u df/dx
        let convection = result.diagonal * field[i_0] - result.neighbors;

        assert!((convection - u).abs() < 1e-4, "{convection}");
    }
}
