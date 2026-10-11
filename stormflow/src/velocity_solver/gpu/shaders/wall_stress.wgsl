// GPU version of `wall_stress_kernel`, applied to all entries in `WallStressEntries` in a single
// dispatch. Each entry targets a unique component of a unique cell of `velocity_star`, and only
// reads `velocity`, so the entries are fully independent. The log-law constants (WALL_KAPPA,
// WALL_E, WALL_Y_PLUS_LAM, WALL_REFERENCE_DISTANCE and NR_FRICTION_VELOCITY_ITERATIONS) are
// generated from the Rust side.

// Matches `GpuWallStressEntry` on the Rust side.
struct WallStressEntry {
    cell_index: u32,
    axis: u32,
    delta: f32,
    tangential_factor: f32,
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

@group(0) @binding(0) var<uniform> grid: Grid;
@group(0) @binding(1) var<uniform> params: VelocityParams;
@group(0) @binding(2) var<storage, read> entries: array<WallStressEntry>;
@group(0) @binding(3) var<storage, read> velocity: array<f32>;
@group(0) @binding(4) var<storage, read_write> velocity_star: array<f32>;

fn sample_reference_velocity(e: u32, component: u32, base: u32) -> f32 {
    let w_offset = 6u * component;

    var sum: f32 = 0.0;

    for (var a: u32 = 0u; a < 2u; a = a + 1u) {
        for (var b: u32 = 0u; b < 2u; b = b + 1u) {
            for (var c: u32 = 0u; c < 2u; c = c + 1u) {
                let idx = base
                    + a * grid.extended_stride.x
                    + b * grid.extended_stride.y
                    + c * grid.extended_stride.z;

                sum += entries[e].weights[w_offset + a] *
                    entries[e].weights[w_offset + 2u + b] *
                    entries[e].weights[w_offset + 4u + c] *
                    velocity[3u * idx + component];
            }
        }
    }

    return sum;
}

// See `WallFunctionConstants::friction_velocity`
fn friction_velocity(tangential_velocity: f32, distance: f32, viscosity: f32) -> f32 {
    if tangential_velocity <= 0.0 {
        return 0.0;
    }

    let laminar_friction_velocity = sqrt(viscosity * tangential_velocity / distance);

    if laminar_friction_velocity * distance / viscosity <= WALL_Y_PLUS_LAM {
        return laminar_friction_velocity;
    }

    var u_tau = laminar_friction_velocity;

    for (var i: u32 = 0u; i < NR_FRICTION_VELOCITY_ITERATIONS; i = i + 1u) {
        let log_term = log(max(WALL_E * distance * u_tau / viscosity, 1.0 + 1e-3));

        u_tau = WALL_KAPPA * tangential_velocity / log_term;
    }

    return u_tau;
}

@compute @workgroup_size(WG_1D)
fn main(
    @builtin(global_invocation_id) gid: vec3<u32>,
    @builtin(num_workgroups) nwg: vec3<u32>
) {
    let e = linear_index(gid, nwg);

    if e >= arrayLength(&entries) {
        return;
    }

    let v = vec3<f32>(
        sample_reference_velocity(e, 0u, entries[e].base_index_u),
        sample_reference_velocity(e, 1u, entries[e].base_index_v),
        sample_reference_velocity(e, 2u, entries[e].base_index_w),
    );

    let normal = vec3<f32>(entries[e].normal_x, entries[e].normal_y, entries[e].normal_z);

    let normal_velocity = dot(v, normal);
    let tangential_velocity = length(v - normal_velocity * normal);

    if tangential_velocity < 1e-12 {
        return;
    }

    let u_tau = friction_velocity(tangential_velocity, WALL_REFERENCE_DISTANCE, params.viscosity);

    let sink_coefficient = u_tau * u_tau * entries[e].delta / tangential_velocity;

    let field_index = 3u * entries[e].cell_index + entries[e].axis;

    velocity_star[field_index] = velocity_star[field_index] /
        (1.0 + params.time_step * sink_coefficient * entries[e].tangential_factor);
}
