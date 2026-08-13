use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

use super::Grid;

#[derive(Debug, Clone, Copy)]
/// A trilinear interpolation stencil: 8 corner samples, stored as a single base flat index plus
/// per-axis weight pairs (the interpolation is separable/tensor-product, so this is equivalent to,
/// but far more compact than, storing all 8 absolute indices and weights directly). The sample
/// at offset `[a, b, c]` (each in `0..2`) from `base_index` is weighted by
/// `weights[0][a] * weights[1][b] * weights[2][c]`.
pub struct TrilinearStencil {
    pub base_index: usize,
    pub weights: [[Float; 2]; 3],
}

impl TrilinearStencil {
    #[inline(always)]
    pub fn sample_component(&self, field: &[SpatialVector], component: usize, stride: [usize; 3]) -> Float {
        let mut sum = 0.0;

        for a in 0..2 {
            for b in 0..2 {
                for c in 0..2 {
                    let idx = self.base_index + a * stride[0] + b * stride[1] + c * stride[2];
                    sum += self.weights[0][a] * self.weights[1][b] * self.weights[2][c] * field[idx][component];
                }
            }
        }

        sum
    }

    #[inline(always)]
    pub fn sample_scalar(&self, field: &[Float], stride: [usize; 3]) -> Float {
        let mut sum = 0.0;

        for a in 0..2 {
            for b in 0..2 {
                for c in 0..2 {
                    let idx = self.base_index + a * stride[0] + b * stride[1] + c * stride[2];
                    sum += self.weights[0][a] * self.weights[1][b] * self.weights[2][c] * field[idx];
                }
            }
        }

        sum
    }
}

#[derive(Debug, Clone, Copy)]
/// A tricubic (4-point Lagrange per axis) interpolation stencil: 64 corner samples, stored the same compact way
/// as `TrilinearStencil` (base flat index + per-axis weight quads). The sample at offset
/// `[a, b, c]` (each in `0..4`) from `base_index` is weighted by
/// `weights[0][a] * weights[1][b] * weights[2][c]`.
pub struct TricubicStencil {
    pub base_index: usize,
    pub weights: [[Float; 4]; 3],
}

impl TricubicStencil {
    #[inline(always)]
    pub fn sample_component(&self, field: &[SpatialVector], component: usize, stride: [usize; 3]) -> Float {
        let mut sum = 0.0;

        for a in 0..4 {
            for b in 0..4 {
                for c in 0..4 {
                    let idx = self.base_index + a * stride[0] + b * stride[1] + c * stride[2];
                    sum += self.weights[0][a] * self.weights[1][b] * self.weights[2][c] * field[idx][component];
                }
            }
        }

        sum
    }

    #[inline(always)]
    pub fn sample_scalar(&self, field: &[Float], stride: [usize; 3]) -> Float {
        let mut sum = 0.0;

        for a in 0..4 {
            for b in 0..4 {
                for c in 0..4 {
                    let idx = self.base_index + a * stride[0] + b * stride[1] + c * stride[2];
                    sum += self.weights[0][a] * self.weights[1][b] * self.weights[2][c] * field[idx];
                }
            }
        }

        sum
    }
}

#[inline(always)]
/// Weights of the 4-point Lagrange cubic through samples at relative offsets `-1, 0, 1, 2` from
/// the sample below `t`, as a function of the fractional position `t` in `[0, 1]` between offsets
/// `0` and `1`. Unlike a Catmull-Rom/Hermite construction (which only matches a true cubic
/// exactly at the symmetric midpoint, since its tangents are central-difference estimates rather
/// than exact derivatives), this is the unique cubic passing through all 4 samples, so it
/// reproduces any cubic field exactly for every `t` — the direct generalization of
/// `convect_and_diffuse::interp4`, which is this same basis evaluated at the fixed `t = 0.5`.
fn cubic_interpolation_weights(t: Float) -> [Float; 4] {
    let t2 = t * t;
    let t3 = t2 * t;

    [
        (-t3 + 3.0 * t2 - 2.0 * t) * (1.0 / 6.0),
        (t3 - 2.0 * t2 - t + 2.0) * 0.5,
        (-t3 + t2 + 2.0 * t) * 0.5,
        (t3 - t) * (1.0 / 6.0),
    ]
}

impl Grid {
    #[inline(always)]
    /// Returns the lower-corner indices (in whichever indexing `shape` describes — extended or
    /// interior) and the fractional weights (in `[0, 1]` on each axis) of the sample pair
    /// enclosing `point`, for a field whose index `[0, 0, 0]` sits at physical location
    /// `field_origin`. `margin_low`/`margin_high` reserve that many extra cells below/above the
    /// returned index (so a stencil spanning `index - margin_low ..= index + 1 + margin_high` is
    /// always in bounds). Points outside the resulting valid range are clamped to the nearest
    /// valid stencil rather than extrapolated indefinitely.
    fn clamped_floor_index_and_fraction(
        &self,
        field_origin: SpatialVector,
        point: SpatialVector,
        shape: [usize; 3],
        margin_low: usize,
        margin_high: usize
    ) -> ([usize; 3], SpatialVector) {
        let mut i0 = [0usize; 3];
        let mut t = SpatialVector::default();

        for axis in 0..3 {
            let raw_index = (point[axis] - field_origin[axis]) * self.inv_cell_length[axis];

            let min_index = margin_low as Float;
            let max_index = (shape[axis] - 1 - margin_high) as Float;

            let clamped_index = raw_index.clamp(min_index, max_index);
            let floor_index = (clamped_index.floor() as usize).clamp(margin_low, shape[axis] - 1 - margin_high);

            i0[axis] = floor_index;
            t[axis] = clamped_index - floor_index as Float;
        }

        (i0, t)
    }

    #[inline(always)]
    /// Builds the trilinear interpolation stencil enclosing `point`, for a field whose extended
    /// index `[0, 0, 0]` sits at physical location `field_origin`.
    pub fn trilinear_stencil_at(&self, field_origin: SpatialVector, point: SpatialVector) -> TrilinearStencil {
        let (i0, t) = self.clamped_floor_index_and_fraction(field_origin, point, self.extended_shape, 0, 1);

        TrilinearStencil {
            base_index: self.flat_index_on_extended_grid(i0),
            weights: [
                [1.0 - t[0], t[0]],
                [1.0 - t[1], t[1]],
                [1.0 - t[2], t[2]],
            ],
        }
    }

    #[inline(always)]
    /// Same as `trilinear_stencil_at`, but for a field stored on the **interior** grid layout (see
    /// `tricubic_stencil_at_interior`'s doc comment for why/when this matters).
    pub fn trilinear_stencil_at_interior(&self, field_origin: SpatialVector, point: SpatialVector) -> TrilinearStencil {
        let (i0, t) = self.clamped_floor_index_and_fraction(field_origin, point, self.interior_shape, 0, 1);

        TrilinearStencil {
            base_index: self.flat_index_on_interior_grid(i0),
            weights: [
                [1.0 - t[0], t[0]],
                [1.0 - t[1], t[1]],
                [1.0 - t[2], t[2]],
            ],
        }
    }

    #[inline(always)]
    /// Builds the tricubic (4-point Lagrange per axis) interpolation stencil enclosing `point`, for a field
    /// whose extended index `[0, 0, 0]` sits at physical location `field_origin`.
    pub fn tricubic_stencil_at(&self, field_origin: SpatialVector, point: SpatialVector) -> TricubicStencil {
        let (i0, t) = self.clamped_floor_index_and_fraction(field_origin, point, self.extended_shape, 1, 2);

        let base_indices = [i0[0] - 1, i0[1] - 1, i0[2] - 1];

        TricubicStencil {
            base_index: self.flat_index_on_extended_grid(base_indices),
            weights: [
                cubic_interpolation_weights(t[0]),
                cubic_interpolation_weights(t[1]),
                cubic_interpolation_weights(t[2]),
            ],
        }
    }

    #[inline(always)]
    /// Same as `tricubic_stencil_at`, but for a field stored on the **interior** grid layout (no
    /// ghost cells, indexed via `flat_index_on_interior_grid`) rather than the extended grid — used
    /// by `MultigridCPU`'s pressure iterate, which has no ghost padding at all. `field_origin` must
    /// therefore be the physical location of interior index `[0, 0, 0]` (i.e. `grid.cell_center([0,
    /// 0, 0])`), not `cell_center_extended`.
    pub fn tricubic_stencil_at_interior(&self, field_origin: SpatialVector, point: SpatialVector) -> TricubicStencil {
        let (i0, t) = self.clamped_floor_index_and_fraction(field_origin, point, self.interior_shape, 1, 2);

        let base_indices = [i0[0] - 1, i0[1] - 1, i0[2] - 1];

        TricubicStencil {
            base_index: self.flat_index_on_interior_grid(base_indices),
            weights: [
                cubic_interpolation_weights(t[0]),
                cubic_interpolation_weights(t[1]),
                cubic_interpolation_weights(t[2]),
            ],
        }
    }

    /// Trilinearly interpolates a scalar field stored at extended-grid cell centers (e.g. a
    /// signed distance function) at an arbitrary physical point.
    pub fn interpolate_cell_centered_scalar(&self, values: &[Float], point: SpatialVector) -> Float {
        let field_origin = self.cell_center_extended([0, 0, 0]);
        let stencil = self.trilinear_stencil_at(field_origin, point);

        stencil.sample_scalar(values, self.extended_stride)
    }

    /// Trilinearly interpolates the face-staggered velocity field at an arbitrary physical point.
    /// Each component is interpolated independently, since `velocity[i][axis]` physically sits at
    /// the positive face of extended cell `i` along `axis` rather than at the cell center.
    pub fn interpolate_velocity(&self, velocity: &[SpatialVector], point: SpatialVector) -> SpatialVector {
        let mut result = SpatialVector::default();

        for axis in 0..3 {
            let mut field_origin = self.cell_center_extended([0, 0, 0]);
            field_origin[axis] += 0.5 * self.cell_length[axis];

            let stencil = self.trilinear_stencil_at(field_origin, point);

            result[axis] = stencil.sample_component(velocity, axis, self.extended_stride);
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_grid() -> Grid {
        Grid::new_direct(
            SpatialVector::new(0.0, 0.0, 0.0),
            SpatialVector::new(1.0, 1.0, 1.0),
            [8, 8, 8],
        )
    }

    #[test]
    fn cubic_interpolation_weights_at_half_match_interp4() {
        let w = cubic_interpolation_weights(0.5);

        assert!((w[0] - (-1.0 / 16.0)).abs() < 1e-6);
        assert!((w[1] - (9.0 / 16.0)).abs() < 1e-6);
        assert!((w[2] - (9.0 / 16.0)).abs() < 1e-6);
        assert!((w[3] - (-1.0 / 16.0)).abs() < 1e-6);
    }

    #[test]
    fn trilinear_stencil_reproduces_linear_field_exactly() {
        let grid = test_grid();

        let f = |p: SpatialVector| 2.0 * p[0] - 3.0 * p[1] + 0.5 * p[2] + 1.0;

        let mut values = vec![0.0; grid.nr_extended_cells()];
        for i in 0..grid.nr_extended_cells() {
            let indices = grid.extended_indices_from_flat_index(i);
            values[i] = f(grid.cell_center_extended(indices));
        }

        let point = SpatialVector::new(3.3, 4.7, 2.1);
        let expected = f(point);
        let actual = grid.interpolate_cell_centered_scalar(&values, point);

        assert!((actual - expected).abs() < 1e-3, "expected {expected}, got {actual}");
    }

    #[test]
    fn tricubic_stencil_reproduces_cubic_field_exactly() {
        let grid = test_grid();

        let f = |p: SpatialVector| {
            p[0].powi(3) - 2.0 * p[0].powi(2) + p[0]
                + p[1].powi(3) - p[1]
                + p[2].powi(3) + 0.5 * p[2].powi(2)
        };

        let mut values = vec![0.0; grid.nr_extended_cells()];
        for i in 0..grid.nr_extended_cells() {
            let indices = grid.extended_indices_from_flat_index(i);
            values[i] = f(grid.cell_center_extended(indices));
        }

        let field_origin = grid.cell_center_extended([0, 0, 0]);
        let point = SpatialVector::new(3.3, 4.7, 2.1);
        let stencil = grid.tricubic_stencil_at(field_origin, point);

        let expected = f(point);
        let actual = stencil.sample_scalar(&values, grid.extended_stride);

        assert!((actual - expected).abs() < 1e-2, "expected {expected}, got {actual}");
    }
}
