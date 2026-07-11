// grid_struct.wgsl will be prepended before the rest of the source during loading

@group(0) @binding(0) var<uniform> grid_fine: Grid;
@group(0) @binding(1) var<uniform> grid_coarse: Grid;
@group(0) @binding(2) var<storage, read> x_fine: array<f32>;
@group(0) @binding(3) var<storage, read> rhs_fine: array<f32>;
@group(0) @binding(4) var<storage, read_write> rhs_coarse: array<f32>;

const RESTRICT_WEIGHT: f32 = 0.125;

fn laplacian_stencil(idx_extended: u32) -> f32 {
    let idx_xp = idx_extended + grid_fine.extended_stride.x;
    let idx_xm = idx_extended - grid_fine.extended_stride.x;
    let idx_yp = idx_extended + grid_fine.extended_stride.y;
    let idx_ym = idx_extended - grid_fine.extended_stride.y;
    let idx_zp = idx_extended + 1u;
    let idx_zm = idx_extended - 1u;

    let off_diag =
          grid_fine.inv_cell_length_squared.x * (x_fine[idx_xp] + x_fine[idx_xm])
        + grid_fine.inv_cell_length_squared.y * (x_fine[idx_yp] + x_fine[idx_ym])
        + grid_fine.inv_cell_length_squared.z * (x_fine[idx_zp] + x_fine[idx_zm]);

    return grid_fine.poisson_diagonal * x_fine[idx_extended] + off_diag;
}

@compute @workgroup_size(4, 4, 4)
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

                let flat_fine_interior = i_f * grid_fine.interior_stride.x
                                        + j_f * grid_fine.interior_stride.y
                                        + k_f;

                let idx_fine_extended = (i_f + 1u) * grid_fine.extended_stride.x
                                       + (j_f + 1u) * grid_fine.extended_stride.y
                                       + (k_f + 1u);

                let ax_i = laplacian_stencil(idx_fine_extended);
                let residual_i = rhs_fine[flat_fine_interior] - ax_i;

                restricted_value = restricted_value + RESTRICT_WEIGHT * residual_i;
            }
        }
    }

    let flat_coarse_interior = i_c * grid_coarse.interior_stride.x
                              + j_c * grid_coarse.interior_stride.y
                              + k_c;

    rhs_coarse[flat_coarse_interior] = restricted_value;
}
