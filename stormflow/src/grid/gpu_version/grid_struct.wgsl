// Struct-only definition of the grid layout (no bound uniform variable), for shaders that need
// more than one Grid instance bound at once (e.g. a fine grid and a coarse grid). Shaders that
// only need a single grid should use grid.wgsl instead, which also declares the binding.
struct Grid {
    start_point:              vec4<f32>,
    cell_length:              vec4<f32>,
    inv_cell_length:          vec4<f32>,
    inv_cell_length_squared:  vec4<f32>,
    poisson_diagonal:         f32,
    poisson_inv_diagonal:     f32,
    _pad0:                    f32,
    _pad1:                    f32,
    extended_shape:           vec4<u32>,
    extended_stride:          vec4<u32>,
    interior_shape:           vec4<u32>,
    interior_stride:          vec4<u32>,
}
