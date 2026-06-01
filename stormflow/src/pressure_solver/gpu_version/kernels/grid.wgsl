// Data structure representing the grid, with variable necessary to extract the information from
// it
struct Grid {
    interior_shape_x: u32,
    interior_shape_y: u32,
    interior_shape_z: u32,
    extended_stride_x: u32,
    extended_stride_y: u32,
    inv_dx2: f32,
    inv_dy2: f32,
    inv_dz2: f32,
    poisson_inv_diagonal: f32,
}

@group(0) @binding(0) var<uniform> grid: Grid;