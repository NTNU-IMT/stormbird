// gpu_grid.wgsl will be prepended before the rest of the source during loading.
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

/// One axis' contribution to the 4th order accurate off-diagonal stencil sum — mirrors
/// jacobi_shader.wgsl's `axis_off_diagonal` (see there for the boundary substitution rule),
/// reading from `x_fine` instead of `current`. Must stay in sync with both that and
/// `multigrid_cpu::kernels::jacobi::axis_off_diagonal_sum`.
fn axis_off_diagonal(idx: u32, stride: u32, position: u32, count: u32, sign_min: f32, sign_max: f32, inv_dx2: f32) -> f32 {
    var u_m1: f32;
    if position >= 1u {
        u_m1 = x_fine[idx - stride];
    } else {
        u_m1 = sign_min * x_fine[idx];
    }

    var u_m2: f32;
    if position >= 2u {
        u_m2 = x_fine[idx - 2u * stride];
    } else if position == 1u {
        u_m2 = sign_min * x_fine[idx - stride];
    } else {
        u_m2 = sign_min * x_fine[idx + stride];
    }

    var u_p1: f32;
    if position + 1u < count {
        u_p1 = x_fine[idx + stride];
    } else {
        u_p1 = sign_max * x_fine[idx];
    }

    var u_p2: f32;
    if position + 2u < count {
        u_p2 = x_fine[idx + 2u * stride];
    } else if position + 1u < count {
        u_p2 = sign_max * x_fine[idx + stride];
    } else {
        u_p2 = sign_max * x_fine[idx - stride];
    }

    return inv_dx2 * ((4.0 / 3.0) * (u_m1 + u_p1) - (1.0 / 12.0) * (u_m2 + u_p2));
}

/// Residual r = rhs - A*x at fine interior cell (ii,ji,ki)/idx, using the same 4th order
/// boundary-folded stencil as jacobi_shader.wgsl's `off_diagonal_sum`.
fn residual_at(idx: u32, ii: u32, ji: u32, ki: u32) -> f32 {
    let nx = grid_fine.interior_shape.x;
    let ny = grid_fine.interior_shape.y;
    let nz = grid_fine.interior_shape.z;

    let sign_x0 = select(1.0, -1.0, zero_value_flag(0u, 0u) == 1u);
    let sign_x1 = select(1.0, -1.0, zero_value_flag(0u, 1u) == 1u);
    let sign_y0 = select(1.0, -1.0, zero_value_flag(1u, 0u) == 1u);
    let sign_y1 = select(1.0, -1.0, zero_value_flag(1u, 1u) == 1u);
    let sign_z0 = select(1.0, -1.0, zero_value_flag(2u, 0u) == 1u);
    let sign_z1 = select(1.0, -1.0, zero_value_flag(2u, 1u) == 1u);

    let off_diag = axis_off_diagonal(idx, grid_fine.interior_stride.x, ii, nx, sign_x0, sign_x1, grid_fine.inv_cell_length_squared.x)
                 + axis_off_diagonal(idx, grid_fine.interior_stride.y, ji, ny, sign_y0, sign_y1, grid_fine.inv_cell_length_squared.y)
                 + axis_off_diagonal(idx, 1u, ki, nz, sign_z0, sign_z1, grid_fine.inv_cell_length_squared.z);

    let ax = grid_fine.poisson_diagonal4 * x_fine[idx] + off_diag;

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