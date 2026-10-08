// Shared by all turbulence solver shaders. gpu_grid.wgsl and the generated consts
// (INTERIOR_OFFSET, the workgroup sizes, UP_AXIS, N_EXTENDED_CELLS, NR_FIELDS,
// WALL_FUNCTION_FIELD and CONVECTION_SCHEME) are prepended before this during loading, see
// `turbulence_solver::gpu`.
//
// The transported fields of the model are stored field-major in one flat array: value `f` of cell
// `i` is at index `f * N_EXTENDED_CELLS + i`. Vector fields are stored with component `c` of cell
// `i` at index `3 * i + c`.

// Matches `TurbulenceParams` on the Rust side
struct TurbulenceParams {
    time_step:          f32,
    // The molecular viscosity
    viscosity:          f32,
    max_eddy_viscosity: f32,
    y_plus_lam:         f32,
    // Wall function constants, where c_mu_25 = c_mu^0.25 and c_mu_75 = c_mu^0.75
    c_mu_25:            f32,
    c_mu_75:            f32,
    kappa:              f32,
    e:                  f32,
}

// Matches `GpuMirrorEntry` on the Rust side
struct MirrorEntry {
    cell_index: u32,
    // Base index of the trilinear stencil of the image point
    base_index: u32,
    // Two weights for each of the three axes
    weights: array<f32, 6>,
}

// Matches `GpuWallFunctionEntry` on the Rust side
struct WallFunctionEntry {
    cell_index: u32,
    wall_distance: f32,
    reference_distance: f32,
    normal_x: f32,
    normal_y: f32,
    normal_z: f32,
    // Base index of the trilinear stencil of each velocity component at the reference point
    base_index_u: u32,
    base_index_v: u32,
    base_index_w: u32,
    // For each velocity component, two weights for each of the three axes
    weights: array<f32, 18>,
}

// Matches `GpuDampingEntry` on the Rust side
struct DampingEntry {
    cell_index: u32,
    mu: f32,
}

// Flat index for kernels that iterate over a 1D list, dispatched with `dispatch_1d` on the Rust
// side, which splits large lists over two dispatch dimensions.
fn linear_index(gid: vec3<u32>, nwg: vec3<u32>) -> u32 {
    return gid.x + gid.y * nwg.x * WG_1D;
}

// Flat extended index of the interior cell of an invocation of a kernel dispatched with
// `dispatch_interior`, or N_EXTENDED_CELLS if the invocation is outside the interior grid.
fn interior_cell_index(gid: vec3<u32>, interior_shape: vec3<u32>, stride: vec3<u32>) -> u32 {
    let ki = gid.x;
    let ji = gid.y;
    let ii = gid.z;

    if ii >= interior_shape.x || ji >= interior_shape.y || ki >= interior_shape.z {
        return N_EXTENDED_CELLS;
    }

    return (ii + INTERIOR_OFFSET) * stride.x + (ji + INTERIOR_OFFSET) * stride.y + (ki + INTERIOR_OFFSET);
}
