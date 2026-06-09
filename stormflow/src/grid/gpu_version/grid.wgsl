// Data structure representing the grid, with variable necessary to extract the information from it
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

@group(0) @binding(0) var<uniform> grid: Grid;
