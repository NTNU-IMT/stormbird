// gpu_grid.wgsl and jacobi_common.wgsl are prepended before this during loading (see
// jacobi_zero_gradient_shader.rs), same as jacobi_shader.wgsl — off_diagonal_sum,
// jacobi_relaxed_update and friends live there, shared between the two. ZERO_GRADIENT_N
// (weights-per-axis: 2 for ZeroGradientInterpolationOrder::Trilinear, 4 for Tricubic) and
// ZERO_GRADIENT_RELAXATION are injected as consts at shader-generation time (see
// zero_gradient_consts_wgsl), matching how BC_X0 etc. are baked in for the boundary condition.
//
// Implements the zero-gradient (Neumann) condition on walls, for the walls selected by
// `ZeroGradientOnWalls`: the cells just inside a wall geometry are blended towards the pressure at
// their mirror image point across the wall surface.
//
// This is a separate pipeline from jacobi_shader.wgsl's, rather than a runtime branch added to it,
// so the plain (disabled) path never binds or touches any of the 4 extra buffers below — see
// `multigrid_cpu::kernels::jacobi::jacobi_kernel_with_zero_gradient`'s doc comment for the CPU
// side of this same design.

@group(0) @binding(0) var<uniform> grid: Grid;
@group(0) @binding(1) var<storage, read> current: array<f32>;
@group(0) @binding(2) var<storage, read> rhs: array<f32>;
@group(0) @binding(3) var<storage, read_write> new_sol: array<f32>;

// Per interior cell (same indexing as current/rhs/new_sol): -1 if uncorrected, otherwise the
// index into weights/base_index/mu below. Matches ZeroGradientStencils::cell_lookup exactly.
@group(0) @binding(4) var<storage, read> cell_lookup: array<i32>;
// Flattened per entry: ZERO_GRADIENT_N weights for each of the 3 axes, laid out
// [axis0_weights(ZERO_GRADIENT_N), axis1_weights(ZERO_GRADIENT_N), axis2_weights(ZERO_GRADIENT_N)].
@group(0) @binding(5) var<storage, read> weights: array<f32>;
// One base flat interior index per entry — the corner the ZERO_GRADIENT_N-wide gather starts from
// along each axis, matching TrilinearStencil/TricubicStencil::base_index.
@group(0) @binding(6) var<storage, read> base_index: array<u32>;
// One blend factor per entry: 0 deep inside the body, approaching 1 near the fluid.
@group(0) @binding(7) var<storage, read> mu: array<f32>;

const WG: u32 = 8u;

/// Tricubic/trilinear image-point sample for entry `entry_index`, gathering ZERO_GRADIENT_N values
/// per axis starting at `base_index[entry_index]` — the same separable tensor-product gather as
/// TrilinearStencil/TricubicStencil::sample_scalar on the CPU side, just reading from flat
/// weights/base_index buffers instead of a Rust struct.
fn sample_mirror_image(entry_index: u32) -> f32 {
    let base = base_index[entry_index];
    let w_offset = entry_index * 3u * ZERO_GRADIENT_N;

    var sum: f32 = 0.0;

    for (var a: u32 = 0u; a < ZERO_GRADIENT_N; a = a + 1u) {
        let wa = weights[w_offset + a];

        for (var b: u32 = 0u; b < ZERO_GRADIENT_N; b = b + 1u) {
            let wb = weights[w_offset + ZERO_GRADIENT_N + b];

            for (var c: u32 = 0u; c < ZERO_GRADIENT_N; c = c + 1u) {
                let wc = weights[w_offset + 2u * ZERO_GRADIENT_N + c];

                let idx = base
                    + a * grid.interior_stride.x
                    + b * grid.interior_stride.y
                    + c * grid.interior_stride.z;

                sum += wa * wb * wc * current[idx];
            }
        }
    }

    return sum;
}

@compute @workgroup_size(WG, WG, WG)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let ii = gid.x;
    let ji = gid.y;
    let ki = gid.z;

    if ii >= grid.interior_shape.x ||
       ji >= grid.interior_shape.y ||
       ki >= grid.interior_shape.z {
        return;
    }

    let idx = ii * grid.interior_stride.x + ji * grid.interior_stride.y + ki;

    let relaxed = jacobi_relaxed_update(idx, ii, ji, ki);

    let lookup_index = cell_lookup[idx];

    if lookup_index < 0 {
        new_sol[idx] = relaxed;
        return;
    }

    let entry_index = u32(lookup_index);
    let p_image = sample_mirror_image(entry_index);
    let mu_value = mu[entry_index];

    let corrected_target = mu_value * relaxed + (1.0 - mu_value) * p_image;

    // Under-relax the correction itself, matching jacobi_kernel_with_zero_gradient exactly:
    // move only partway from the cell's own previous value toward the blended target.
    new_sol[idx] = (1.0 - ZERO_GRADIENT_RELAXATION) * current[idx] + ZERO_GRADIENT_RELAXATION * corrected_target;
}
