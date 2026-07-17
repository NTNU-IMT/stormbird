// grid.wgsl will be prepended before this source during loading.
// BC_X0/BC_X1/BC_Y0/BC_Y1/BC_Z0/BC_Z1 (0 = ZeroGradient, 1 = ZeroValue) are injected as plain
// WGSL consts, same as jacobi_shader.wgsl.
//
// Runs once per `solve()` call (not per iteration): copies the interior-only solve result `x`
// into its offset position in the extended layout `Simulation::update_velocity` and
// `export_fields_as_vtk` expect, and computes each boundary-adjacent cell's extrapolated ghost
// value(s) directly (up to 3 per thread, for a domain-corner interior cell) using the same
// `±value` substitution already used by the stencils, instead of a separate CPU pass. Corner/edge
// cells of the extended array (where two or more indices are out of range) are left untouched,
// since nothing ever reads them.

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

@compute @workgroup_size(4, 4, 4)
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

    let idx_extended = (ii + 1u) * grid.extended_stride.x
                      + (ji + 1u) * grid.extended_stride.y
                      + (ki + 1u);

    solution[idx_extended] = value;

    let nx = grid.interior_shape.x;
    let ny = grid.interior_shape.y;
    let nz = grid.interior_shape.z;

    if ii == 0u {
        let sign = select(1.0, -1.0, zero_value_flag(0u, 0u) == 1u);
        solution[idx_extended - grid.extended_stride.x] = sign * value;
    }
    if ii + 1u == nx {
        let sign = select(1.0, -1.0, zero_value_flag(0u, 1u) == 1u);
        solution[idx_extended + grid.extended_stride.x] = sign * value;
    }

    if ji == 0u {
        let sign = select(1.0, -1.0, zero_value_flag(1u, 0u) == 1u);
        solution[idx_extended - grid.extended_stride.y] = sign * value;
    }
    if ji + 1u == ny {
        let sign = select(1.0, -1.0, zero_value_flag(1u, 1u) == 1u);
        solution[idx_extended + grid.extended_stride.y] = sign * value;
    }

    if ki == 0u {
        let sign = select(1.0, -1.0, zero_value_flag(2u, 0u) == 1u);
        solution[idx_extended - 1u] = sign * value;
    }
    if ki + 1u == nz {
        let sign = select(1.0, -1.0, zero_value_flag(2u, 1u) == 1u);
        solution[idx_extended + 1u] = sign * value;
    }
}
