// grid_struct.wgsl will be prepended before the rest of the source during loading.
// `x_fine`/`x_coarse` are interior-sized (no ghost cells).

@group(0) @binding(0) var<uniform> grid_fine: Grid;
@group(0) @binding(1) var<uniform> grid_coarse: Grid;
@group(0) @binding(2) var<storage, read> x_coarse: array<f32>;
@group(0) @binding(3) var<storage, read_write> x_fine: array<f32>;

@compute @workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i_f = gid.x;
    let j_f = gid.y;
    let k_f = gid.z;

    if i_f >= grid_fine.interior_shape.x ||
       j_f >= grid_fine.interior_shape.y ||
       k_f >= grid_fine.interior_shape.z {
        return;
    }

    let idx_fine = i_f * grid_fine.interior_stride.x
                 + j_f * grid_fine.interior_stride.y
                 + k_f;

    let nx_c = grid_coarse.interior_shape.x;
    let ny_c = grid_coarse.interior_shape.y;
    let nz_c = grid_coarse.interior_shape.z;

    // Fine cell center position in "coarse cell units", relative to coarse cell centers.
    let xi   = f32(i_f) * 0.5 - 0.25;
    let eta  = f32(j_f) * 0.5 - 0.25;
    let zeta = f32(k_f) * 0.5 - 0.25;

    let i_c_base = u32(clamp(floor(xi),   0.0, f32(nx_c - 1u)));
    let j_c_base = u32(clamp(floor(eta),  0.0, f32(ny_c - 1u)));
    let k_c_base = u32(clamp(floor(zeta), 0.0, f32(nz_c - 1u)));

    let sx = clamp(xi   - f32(i_c_base), 0.0, 1.0);
    let sy = clamp(eta  - f32(j_c_base), 0.0, 1.0);
    let sz = clamp(zeta - f32(k_c_base), 0.0, 1.0);

    let wx = vec2<f32>(1.0 - sx, sx);
    let wy = vec2<f32>(1.0 - sy, sy);
    let wz = vec2<f32>(1.0 - sz, sz);

    var correction_value: f32 = 0.0;

    for (var di: u32 = 0u; di < 2u; di = di + 1u) {
        let i_c = min(i_c_base + di, nx_c - 1u);

        for (var dj: u32 = 0u; dj < 2u; dj = dj + 1u) {
            let j_c = min(j_c_base + dj, ny_c - 1u);

            for (var dk: u32 = 0u; dk < 2u; dk = dk + 1u) {
                let k_c = min(k_c_base + dk, nz_c - 1u);

                let weight = wx[di] * wy[dj] * wz[dk];

                let idx_coarse = i_c * grid_coarse.interior_stride.x
                                + j_c * grid_coarse.interior_stride.y
                                + k_c;

                correction_value = correction_value + weight * x_coarse[idx_coarse];
            }
        }
    }

    x_fine[idx_fine] = x_fine[idx_fine] + correction_value;
}
