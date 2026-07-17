use std::f64::consts::PI;

use oxifft::rdft::solvers::{R2rKind, R2rSolver};
use rayon::prelude::*;
use stormath::type_aliases::Float;

use crate::pressure_solver::boundary_conditions::PressureBoundaryCondition;

/// Which pair of trigonometric transforms diagonalizes the 1D Laplacian stencil along an axis,
/// selected from the two boundary conditions on that axis' faces.
///
/// The mapping follows from how `boundary_conditions::set_ghost_cells_kernel`/`off_diagonal_sum`
/// fold each boundary condition into the stencil: `ZeroGradient` mirrors the neighbor value
/// (symmetric extension, "even" about the face) while `ZeroValue` mirrors its negation
/// (antisymmetric extension, "odd" about the face). FFTW's REDFT/RODFT family is defined by
/// exactly this even/odd-about-each-endpoint classification (see FFTW docs, "1d Real-even/odd
/// DFTs"), which is why the transform choice below matches FFTW's REDFT10/RODFT10/REDFT11/RODFT11
/// naming: even-even -> DCT-II/III, odd-odd -> DST-II/III, even-odd -> DCT-IV (self-inverse),
/// odd-even -> DST-IV (self-inverse).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxisTransformKind {
    /// Both faces `ZeroGradient`.
    NeumannNeumann,
    /// Both faces `ZeroValue`.
    DirichletDirichlet,
    /// Face 0 (negative) `ZeroGradient`, face 1 (positive) `ZeroValue`.
    NeumannDirichlet,
    /// Face 0 (negative) `ZeroValue`, face 1 (positive) `ZeroGradient`.
    DirichletNeumann,
}

/// Forward or inverse pass; see `AxisTransformKind::forward`/`inverse`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformDirection {
    Forward,
    Inverse,
}

impl AxisTransformKind {
    pub fn from_faces(face0: PressureBoundaryCondition, face1: PressureBoundaryCondition) -> Self {
        match (face0, face1) {
            (PressureBoundaryCondition::ZeroGradient, PressureBoundaryCondition::ZeroGradient) => Self::NeumannNeumann,
            (PressureBoundaryCondition::ZeroValue, PressureBoundaryCondition::ZeroValue) => Self::DirichletDirichlet,
            (PressureBoundaryCondition::ZeroGradient, PressureBoundaryCondition::ZeroValue) => Self::NeumannDirichlet,
            (PressureBoundaryCondition::ZeroValue, PressureBoundaryCondition::ZeroGradient) => Self::DirichletNeumann,
        }
    }

    /// Builds the `R2rSolver` this axis should use, sized for `n` (the axis' cell count). Meant to
    /// be built once (in `FftCPU::new`) and reused for every line and every `solve()` call — see
    /// `forward`/`inverse` for why a single cached solver is enough for all four transform kinds.
    ///
    /// The `R2rKind` passed to `R2rSolver::new` is otherwise irrelevant here: `R2rSolver::new`
    /// unconditionally builds *both* the DCT-II/III plan and twiddle table and the DCT-IV plan and
    /// twiddle table regardless of `kind` (confirmed in `oxifft` 0.3.2's `types_r2r.rs`), and we
    /// only ever call the kind-agnostic `execute_dct2`/`execute_dct3`/`execute_dct4` methods below
    /// (never the `kind`-dispatching `execute`), so any `R2rKind` variant works equally.
    pub fn build_solver(n: usize) -> R2rSolver<Float> {
        R2rSolver::new(R2rKind::Redft10, n)
    }

    /// Forward transform for this axis, using a solver built by `build_solver` for this axis'
    /// length. `aux_a`/`aux_b` are scratch buffers of the same length as `input`/`output`, reused
    /// across calls by the caller (see `transform_along_axis`) to avoid per-line allocation.
    ///
    /// For `DirichletDirichlet`/`DirichletNeumann` this replicates the DST-II/DST-IV-via-DCT
    /// reduction that `oxifft`'s own `execute_dst2`/`execute_dst4` use internally — but calling
    /// *our* cached `solver` instead of the fresh helper solver those methods construct on every
    /// single call (verified in `oxifft` 0.3.2's `r2r.rs`: `execute_dst2_fast`/`execute_dst3_fast`/
    /// `execute_dst4_fast` each do `let helper = Self::new(...)` internally, rebuilding an FFT plan
    /// and twiddle tables from scratch every call). Bypassing that is the whole point of caching
    /// `solver` in `FftCPU` rather than calling `oxifft::reodft::dst_ii`/`dst_iv` directly.
    #[inline]
    pub fn forward(&self, solver: &R2rSolver<Float>, input: &[Float], output: &mut [Float], aux_a: &mut [Float], aux_b: &mut [Float]) {
        match self {
            Self::NeumannNeumann => solver.execute_dct2(input, output),
            Self::NeumannDirichlet => solver.execute_dct4(input, output),
            Self::DirichletDirichlet => {
                // DST-II(x)[k] = DCT-II(y)[N-1-k], y[n] = (-1)^n * x[n].
                let n = input.len();
                for i in 0..n {
                    aux_a[i] = if i % 2 == 0 { input[i] } else { -input[i] };
                }
                solver.execute_dct2(aux_a, aux_b);
                for k in 0..n {
                    output[k] = aux_b[n - 1 - k];
                }
            },
            Self::DirichletNeumann => dst_iv_via_cached_dct4(solver, input, output, aux_a, aux_b),
        }
    }

    /// Inverse of `forward`. For the mixed cases this is the *same* transform (DCT-IV/DST-IV are
    /// self-inverse); for the pure cases it's the paired type-III transform.
    #[inline]
    pub fn inverse(&self, solver: &R2rSolver<Float>, input: &[Float], output: &mut [Float], aux_a: &mut [Float], aux_b: &mut [Float]) {
        match self {
            Self::NeumannNeumann => solver.execute_dct3(input, output),
            Self::NeumannDirichlet => solver.execute_dct4(input, output),
            Self::DirichletDirichlet => {
                // DST-III(f)[n] = (-1)^n * DCT-III(f_reversed)[n], f_reversed[k] = f[N-1-k].
                let n = input.len();
                for k in 0..n {
                    aux_a[k] = input[n - 1 - k];
                }
                solver.execute_dct3(aux_a, aux_b);
                for i in 0..n {
                    output[i] = if i % 2 == 0 { aux_b[i] } else { -aux_b[i] };
                }
            },
            Self::DirichletNeumann => dst_iv_via_cached_dct4(solver, input, output, aux_a, aux_b),
        }
    }

    /// The discrete wavenumber `theta` for mode `k` (of `n`) that the eigenvalue formula
    /// `-4/h^2 * sin^2(theta/2)` is evaluated at. Verified against direct dense linear-algebra
    /// solves of the boundary-folded stencil for all four combinations (see the FFT-vs-multigrid
    /// test in `tests/fft_pressure_solver_smoke.rs`).
    #[inline]
    fn theta(&self, k: usize, n: usize) -> Float {
        let n_f = n as f64;
        let theta = match self {
            Self::NeumannNeumann => (k as f64) * PI / n_f,
            Self::DirichletDirichlet => ((k + 1) as f64) * PI / n_f,
            Self::NeumannDirichlet | Self::DirichletNeumann => ((k as f64) + 0.5) * PI / n_f,
        };
        theta as Float
    }

    /// Per-axis eigenvalues of the boundary-folded 1D Laplacian stencil, one per mode `k = 0..n`,
    /// in the basis this transform diagonalizes it in.
    pub fn eigenvalues(&self, n: usize, inv_cell_length_squared: Float) -> Vec<Float> {
        (0..n)
            .map(|k| {
                let half_theta = 0.5 * self.theta(k, n);
                -4.0 * inv_cell_length_squared * half_theta.sin() * half_theta.sin()
            })
            .collect()
    }
}

/// DST-IV(x)[k] = (-1)^k * DCT-IV(x_reversed)[k], x_reversed[n] = x[N-1-n]. Self-inverse, so this
/// same procedure serves as both `forward` and `inverse` for the `DirichletNeumann` axis kind.
#[inline]
fn dst_iv_via_cached_dct4(solver: &R2rSolver<Float>, input: &[Float], output: &mut [Float], aux_a: &mut [Float], aux_b: &mut [Float]) {
    let n = input.len();
    for i in 0..n {
        aux_a[i] = input[n - 1 - i];
    }
    solver.execute_dct4(aux_a, aux_b);
    for k in 0..n {
        output[k] = if k % 2 == 0 { aux_b[k] } else { -aux_b[k] };
    }
}

/// Applies the forward or inverse transform of `axis_transform` (using the cached `solver`)
/// independently to every 1D line along `axis` of an interior-grid-shaped array, in place.
///
/// Lines are processed in parallel via Rayon, one per task. (An earlier version batched several
/// lines per task to amortize the per-line scratch-buffer allocations below; measured against
/// this simpler per-line version, that was a net regression in most cases — see the comment
/// inside this function.) Each line owns disjoint data indices (`base + a * axis_stride` for
/// `a in 0..axis_len`), so concurrent writes never alias.
pub fn transform_along_axis(
    interior_shape: [usize; 3],
    interior_stride: [usize; 3],
    axis: usize,
    data: &mut [Float],
    axis_transform: AxisTransformKind,
    solver: &R2rSolver<Float>,
    direction: TransformDirection,
) {
    let other_axes = match axis {
        0 => [1, 2],
        1 => [0, 2],
        _ => [0, 1],
    };

    let axis_len = interior_shape[axis];
    let axis_stride = interior_stride[axis];
    let n_outer = interior_shape[other_axes[0]];
    let n_inner = interior_shape[other_axes[1]];
    let outer_stride = interior_stride[other_axes[0]];
    let inner_stride = interior_stride[other_axes[1]];

    let data_ptr = data.as_mut_ptr() as usize;

    // Note: an earlier version of this function batched several lines per Rayon task (amortizing
    // the line_in/line_out/aux_a/aux_b allocations below over many lines) on the theory that
    // per-line allocation was a meaningful cost. Measured head-to-head against this simpler
    // per-line `for_each`, that batching was a net *regression* in most cases (coarser task
    // granularity apparently costs more than the allocations save) — so this stays per-line.
    // (A `copy_from_slice`-based fast path for the contiguous z-axis case was also tried here and
    // measured indistinguishable from noise — run-to-run variance on the dev machine exceeded the
    // apparent effect — so it wasn't kept.)
    (0..n_outer * n_inner).into_par_iter().for_each(|flat| {
        let i_outer = flat / n_inner;
        let i_inner = flat % n_inner;
        let base = i_outer * outer_stride + i_inner * inner_stride;

        let mut line_in = vec![0.0 as Float; axis_len];
        for a in 0..axis_len {
            unsafe {
                line_in[a] = *(data_ptr as *const Float).add(base + a * axis_stride);
            }
        }

        let mut line_out = vec![0.0 as Float; axis_len];
        let mut aux_a = vec![0.0 as Float; axis_len];
        let mut aux_b = vec![0.0 as Float; axis_len];

        match direction {
            TransformDirection::Forward => axis_transform.forward(solver, &line_in, &mut line_out, &mut aux_a, &mut aux_b),
            TransformDirection::Inverse => axis_transform.inverse(solver, &line_in, &mut line_out, &mut aux_a, &mut aux_b),
        }

        for a in 0..axis_len {
            unsafe {
                *(data_ptr as *mut Float).add(base + a * axis_stride) = line_out[a];
            }
        }
    });
}
