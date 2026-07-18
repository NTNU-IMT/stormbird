// grid_struct.wgsl will be prepended before the rest of the source during loading.
// BC_X0/BC_X1/BC_Y0/BC_Y1/BC_Z0/BC_Z1 (0 = ZeroGradient, 1 = ZeroValue) are injected as plain
// WGSL consts at shader-generation time (see jacobi_shader.wgsl for why this boundary-folded
// approach replaces ghost cells). `x_fine`/`rhs_fine` are interior-sized.

@group(0) @binding(0) var<uniform> grid_fine: Grid;
@group(0) @binding(1) var<uniform> grid_coarse: Grid;
@group(0) @binding(2) var<storage, read> x_fine: array<f32>;
@group(0) @binding(3) var<storage, read> rhs_fine: array<f32>;
@group(0) @binding(4) var<storage, read_write> rhs_coarse: array<f32>;

const RESTRICT_WEIGHT: f32 = 0.125;

fn zero_value_flag(axis: u32, face: u32) -> u32 {
    if axis == 0u {
        return select(BC_X0, BC_X1, face == 1u);
    } else if axis == 1u {
        return select(BC_Y0, BC_Y1, face == 1u);
    } else {
        return select(BC_Z0, BC_Z1, face == 1u);
    }
}

/// Residual r = rhs - A*x at fine interior cell (ii,ji,ki)/idx, with a missing neighbor on any
/// axis explicitly substituted by `x_fine[idx]` itself (signed per that face's boundary
/// condition) rather than folded into the diagonal — see jacobi_shader.wgsl's
/// `off_diagonal_sum` for why this has to be an explicit substitution, not an implicit one.
fn residual_at(idx: u32, ii: u32, ji: u32, ki: u32) -> f32 {
    let nx = grid_fine.interior_shape.x;
    let ny = grid_fine.interior_shape.y;
    let nz = grid_fine.interior_shape.z;

    var off_diag: f32 = 0.0;

    if ii > 0u {
        off_diag += grid_fine.inv_cell_length_squared.x * x_fine[idx - grid_fine.interior_stride.x];
    } else {
        off_diag += grid_fine.inv_cell_length_squared.x * select(1.0, -1.0, zero_value_flag(0u, 0u) == 1u) * x_fine[idx];
    }
    if ii + 1u < nx {
        off_diag += grid_fine.inv_cell_length_squared.x * x_fine[idx + grid_fine.interior_stride.x];
    } else {
        off_diag += grid_fine.inv_cell_length_squared.x * select(1.0, -1.0, zero_value_flag(0u, 1u) == 1u) * x_fine[idx];
    }

    if ji > 0u {
        off_diag += grid_fine.inv_cell_length_squared.y * x_fine[idx - grid_fine.interior_stride.y];
    } else {
        off_diag += grid_fine.inv_cell_length_squared.y * select(1.0, -1.0, zero_value_flag(1u, 0u) == 1u) * x_fine[idx];
    }
    if ji + 1u < ny {
        off_diag += grid_fine.inv_cell_length_squared.y * x_fine[idx + grid_fine.interior_stride.y];
    } else {
        off_diag += grid_fine.inv_cell_length_squared.y * select(1.0, -1.0, zero_value_flag(1u, 1u) == 1u) * x_fine[idx];
    }

    if ki > 0u {
        off_diag += grid_fine.inv_cell_length_squared.z * x_fine[idx - 1u];
    } else {
        off_diag += grid_fine.inv_cell_length_squared.z * select(1.0, -1.0, zero_value_flag(2u, 0u) == 1u) * x_fine[idx];
    }
    if ki + 1u < nz {
        off_diag += grid_fine.inv_cell_length_squared.z * x_fine[idx + 1u];
    } else {
        off_diag += grid_fine.inv_cell_length_squared.z * select(1.0, -1.0, zero_value_flag(2u, 1u) == 1u) * x_fine[idx];
    }

    let ax = grid_fine.poisson_diagonal * x_fine[idx] + off_diag;

    return rhs_fine[idx] - ax;
}

@compute @workgroup_size(8, 8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i_c = gid.x;
    let j_c = gid.y;
    let k_c = gid.z;

    if i_c >= grid_coarse.interior_shape.x ||
       j_c >= grid_coarse.interior_shape.y ||
       k_c >= grid_coarse.interior_shape.z {
        return;
    }

    let base_i_f = 2u * i_c;
    let base_j_f = 2u * j_c;
    let base_k_f = 2u * k_c;

    var restricted_value: f32 = 0.0;

    for (var di: u32 = 0u; di < 2u; di = di + 1u) {
        for (var dj: u32 = 0u; dj < 2u; dj = dj + 1u) {
            for (var dk: u32 = 0u; dk < 2u; dk = dk + 1u) {
                let i_f = base_i_f + di;
                let j_f = base_j_f + dj;
                let k_f = base_k_f + dk;

                let idx_fine = i_f * grid_fine.interior_stride.x
                             + j_f * grid_fine.interior_stride.y
                             + k_f;

                let residual = residual_at(idx_fine, i_f, j_f, k_f);

                restricted_value = restricted_value + RESTRICT_WEIGHT * residual;
            }
        }
    }

    let flat_coarse_interior = i_c * grid_coarse.interior_stride.x
                              + j_c * grid_coarse.interior_stride.y
                              + k_c;

    rhs_coarse[flat_coarse_interior] = restricted_value;
}