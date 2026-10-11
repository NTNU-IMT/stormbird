// gpu_grid.wgsl will be prepended before this source during loading.
// BC_X0/BC_X1/BC_Y0/BC_Y1/BC_Z0/BC_Z1 (0 = ZeroGradient, 1 = ZeroValue) are injected as plain
// WGSL consts, same as jacobi_shader.wgsl.
//
// Runs once per `solve()` call (not per iteration): copies the interior-only solve result `x`
// into its offset position in the extended layout `Simulation::update_velocity` and
// `export_fields_as_vtk` expect, and computes each boundary-adjacent cell's extrapolated ghost
// values (up to 3 * INTERIOR_OFFSET per thread, for a domain-corner interior cell), mirroring
// `PressureBoundaryConditions::set_ghost_cells_kernel`: ghost layer `l` (0 = nearest the
// interior) is paired with the interior cell `l` cells in from that same face, signed per the
// face's boundary condition. Corner/edge cells of the extended array (where two or more indices
// are out of range) are left untouched, since nothing ever reads them.

@group(0) @binding(0) var<uniform> grid: Grid;
@group(0) @binding(1) var<storage, read> x: array<f32>;
@group(0) @binding(2) var<storage, read_write> solution: array<f32>;

fn zero_value_flag(axis: u32, face: u32) -> u32 {
    if axis == 0u {
        return select(BC_X0, BC_X1, face == 1u);
    } else if axis == 1u {
        return select(BC_Y0, BC_Y1, face == 1u);
    } else {
        return select(BC_Z0, BC_Z1, face == 1u);
    }
}

@compute @workgroup_size(8, 8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let ii = gid.x;
    let ji = gid.y;
    let ki = gid.z;

    if ii >= grid.interior_shape.x ||
       ji >= grid.interior_shape.y ||
       ki >= grid.interior_shape.z {
        return;
    }

    let idx_interior = ii * grid.interior_stride.x + ji * grid.interior_stride.y + ki;
    let value = x[idx_interior];

    let idx_extended = (ii + INTERIOR_OFFSET) * grid.extended_stride.x
                      + (ji + INTERIOR_OFFSET) * grid.extended_stride.y
                      + (ki + INTERIOR_OFFSET);

    solution[idx_extended] = value;

    let nx = grid.interior_shape.x;
    let ny = grid.interior_shape.y;
    let nz = grid.interior_shape.z;

    if ii == 0u {
        let sign = select(1.0, -1.0, zero_value_flag(0u, 0u) == 1u);
        for (var l: u32 = 0u; l < INTERIOR_OFFSET; l = l + 1u) {
            let neighbor = x[l * grid.interior_stride.x + ji * grid.interior_stride.y + ki];
            solution[idx_extended - (l + 1u) * grid.extended_stride.x] = sign * neighbor;
        }
    }
    if ii + 1u == nx {
        let sign = select(1.0, -1.0, zero_value_flag(0u, 1u) == 1u);
        for (var l: u32 = 0u; l < INTERIOR_OFFSET; l = l + 1u) {
            let neighbor = x[(nx - 1u - l) * grid.interior_stride.x + ji * grid.interior_stride.y + ki];
            solution[idx_extended + (l + 1u) * grid.extended_stride.x] = sign * neighbor;
        }
    }

    if ji == 0u {
        let sign = select(1.0, -1.0, zero_value_flag(1u, 0u) == 1u);
        for (var l: u32 = 0u; l < INTERIOR_OFFSET; l = l + 1u) {
            let neighbor = x[ii * grid.interior_stride.x + l * grid.interior_stride.y + ki];
            solution[idx_extended - (l + 1u) * grid.extended_stride.y] = sign * neighbor;
        }
    }
    if ji + 1u == ny {
        let sign = select(1.0, -1.0, zero_value_flag(1u, 1u) == 1u);
        for (var l: u32 = 0u; l < INTERIOR_OFFSET; l = l + 1u) {
            let neighbor = x[ii * grid.interior_stride.x + (ny - 1u - l) * grid.interior_stride.y + ki];
            solution[idx_extended + (l + 1u) * grid.extended_stride.y] = sign * neighbor;
        }
    }

    if ki == 0u {
        let sign = select(1.0, -1.0, zero_value_flag(2u, 0u) == 1u);
        for (var l: u32 = 0u; l < INTERIOR_OFFSET; l = l + 1u) {
            let neighbor = x[ii * grid.interior_stride.x + ji * grid.interior_stride.y + l];
            solution[idx_extended - (l + 1u)] = sign * neighbor;
        }
    }
    if ki + 1u == nz {
        let sign = select(1.0, -1.0, zero_value_flag(2u, 1u) == 1u);
        for (var l: u32 = 0u; l < INTERIOR_OFFSET; l = l + 1u) {
            let neighbor = x[ii * grid.interior_stride.x + ji * grid.interior_stride.y + (nz - 1u - l)];
            solution[idx_extended + (l + 1u)] = sign * neighbor;
        }
    }
}
