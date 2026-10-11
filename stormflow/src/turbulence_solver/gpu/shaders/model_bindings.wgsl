// The bindings shared by all the entry points of all the model shaders. Each entry point only uses
// a subset of them. Prepended to transport.wgsl and the model shader, see
// `TurbulenceSolverGPU::new`.

@group(0) @binding(0) var<uniform> grid: Grid;
@group(0) @binding(1) var<uniform> params: TurbulenceParams;
@group(0) @binding(2) var<storage, read> velocity: array<f32>;
@group(0) @binding(3) var<storage, read_write> eddy_viscosity: array<f32>;
// The current iterate of the transported fields
@group(0) @binding(4) var<storage, read> fields: array<f32>;
// The transported fields at the start of the time step
@group(0) @binding(5) var<storage, read> fields_old: array<f32>;
// The result of the current Jacobi iteration
@group(0) @binding(6) var<storage, read_write> fields_next: array<f32>;
@group(0) @binding(7) var<storage, read_write> auxiliary_fields: array<f32>;
@group(0) @binding(8) var<storage, read> wall_entries: array<WallFunctionEntry>;
// The fixed values of the wall function field, for each wall function entry
@group(0) @binding(9) var<storage, read_write> wall_values: array<f32>;
