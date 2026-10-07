// Shared by all velocity solver shaders. gpu_grid.wgsl and the generated consts (INTERIOR_OFFSET,
// UP_AXIS, SLIP_N, N_EXTENDED_CELLS and the workgroup sizes) are prepended before this during
// loading, see `velocity_solver::gpu::kernels`.
//
// Vector fields are stored as flat f32 arrays, with component `c` of cell `i` at index `3 * i + c`.

// Matches `VelocityParams` on the Rust side. All derived values are computed on the CPU, in the same
// way as in the CPU version of the solver.
struct VelocityParams {
    time_step:      f32,
    viscosity:      f32,
    inv_density:    f32,
    // density / time_step, as used for the right hand side of the pressure equation
    rhs_scale:      f32,
    // time_step / density, as used when adding the pressure gradient
    gradient_scale: f32,
    _pad0:          f32,
    _pad1:          f32,
    _pad2:          f32,
}

// Flat index for kernels that iterate over a 1D list, dispatched with `dispatch_1d` on the Rust
// side, which splits large lists over two dispatch dimensions.
fn linear_index(gid: vec3<u32>, nwg: vec3<u32>) -> u32 {
    return gid.x + gid.y * nwg.x * WG_1D;
}

// 4th order accurate interpolation onto the midpoint between `f_0` and `f_1`. See
// `convect_and_diffuse::interp4`.
fn interp4(f_m1: f32, f_0: f32, f_1: f32, f_2: f32) -> f32 {
    return (9.0 * (f_0 + f_1) - (f_m1 + f_2)) * (1.0 / 16.0);
}
