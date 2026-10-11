// Struct-only definition of the grid layout (no bound uniform variable). Shaders declare their
// own `var<uniform>` binding(s) against this `Grid` type after it's prepended — one binding for
// shaders that only need a single grid, two (e.g. `grid_fine`/`grid_coarse`) for shaders that
// need more than one Grid instance bound at once.
struct Grid {
    start_point:              vec4<f32>,
    cell_length:              vec4<f32>,
    inv_cell_length:          vec4<f32>,
    inv_cell_length_squared:  vec4<f32>,
    _pad0:                    f32,
    _pad1:                    f32,
    poisson_diagonal4:        f32,
    poisson_inv_diagonal4:    f32,
    _pad2:                    f32,
    _pad3:                    f32,
    _pad4:                    f32,
    _pad5:                    f32,
    extended_shape:           vec4<u32>,
    extended_stride:          vec4<u32>,
    interior_shape:           vec4<u32>,
    interior_stride:          vec4<u32>,
}
