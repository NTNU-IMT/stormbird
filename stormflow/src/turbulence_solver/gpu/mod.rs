//! GPU version of the turbulence solver.
//!
//! All fields are stored on the device, in the same layout as in the CPU version. The velocity and
//! the eddy viscosity are the buffers of the velocity solver, which runs on the same device, so the
//! turbulence solver never transfers any data during the simulation. The kernels mirror the CPU
//! version in [`super::cpu`] one-to-one, and use the same precomputed data from
//! [`TurbulenceSolverSetup`].

use stormath::type_aliases::Float;

use crate::gpu_interface::{
    context::GpuContext,
    kernels::{
        Kernel,
        Binding,
        common_constants_wgsl,
        dispatch_interior,
        dispatch_1d,
        dispatch_plane,
    },
};
use crate::grid::Grid;

use super::TurbulenceSolverSetup;
use super::boundary_conditions::ScalarBoundaryCondition;
use super::wall_treatment::WallTreatmentEntries;

const COMMON_SRC: &str = include_str!("shaders/common.wgsl");
const MODEL_BINDINGS_SRC: &str = include_str!("shaders/model_bindings.wgsl");
const TRANSPORT_SRC: &str = include_str!("shaders/transport.wgsl");
const GHOST_CELLS_SRC: &str = include_str!("shaders/ghost_cells.wgsl");
const MIRROR_SRC: &str = include_str!("shaders/mirror.wgsl");
const FIXED_VALUES_SRC: &str = include_str!("shaders/fixed_values.wgsl");
const DAMPING_SRC: &str = include_str!("shaders/damping.wgsl");

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
/// Matches `TurbulenceParams` in common.wgsl
struct TurbulenceParams {
    time_step: f32,
    viscosity: f32,
    max_eddy_viscosity: f32,
    y_plus_lam: f32,
    c_mu_25: f32,
    c_mu_75: f32,
    kappa: f32,
    e: f32,
}

impl TurbulenceParams {
    fn new(time_step: Float, setup: &TurbulenceSolverSetup) -> Self {
        let constants = &setup.wall_function_constants;

        Self {
            time_step,
            viscosity: setup.viscosity,
            max_eddy_viscosity: setup.max_eddy_viscosity,
            y_plus_lam: constants.y_plus_lam(),
            c_mu_25: constants.c_mu.powf(0.25),
            c_mu_75: constants.c_mu.powf(0.75),
            kappa: constants.kappa,
            e: constants.e,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
/// Matches `MirrorEntry` in common.wgsl
struct GpuMirrorEntry {
    cell_index: u32,
    base_index: u32,
    weights: [f32; 6],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
/// Matches `WallFunctionEntry` in common.wgsl
struct GpuWallFunctionEntry {
    cell_index: u32,
    wall_distance: f32,
    reference_distance: f32,
    normal: [f32; 3],
    base_index: [u32; 3],
    weights: [f32; 18],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
/// Matches `DampingEntry` in common.wgsl
struct GpuDampingEntry {
    cell_index: u32,
    mu: f32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
/// Matches `FaceParams` in the turbulence solver's ghost_cells.wgsl
struct GpuScalarFaceParams {
    axis: u32,
    face: u32,
    condition: u32,
    nr_fields: u32,
}

#[derive(Debug, Clone, Copy)]
/// The buffers with transported fields that the corrections and boundary conditions are applied to
enum Target {
    Fields = 0,
    FieldsNext = 1,
}

struct TurbulenceKernels {
    auxiliary_fields: Kernel,
    wall_functions: Kernel,
    transport: Kernel,
    eddy_viscosity: Kernel,
    ghost_cells: Kernel,
    mirror_compute: Kernel,
    mirror_scatter: Kernel,
    fixed_values: Kernel,
    damping: Kernel,
}

impl TurbulenceKernels {
    fn new(context: &GpuContext, prelude: &str, model_src: &str) -> Self {
        use Binding::*;

        let model_bindings = [
            Uniform, Uniform, ReadOnly, ReadWrite, ReadOnly, ReadOnly, ReadWrite, ReadWrite, ReadOnly, ReadWrite
        ];

        let mirror_bindings = [Uniform, ReadOnly, ReadWrite, ReadWrite];

        Self {
            auxiliary_fields: Kernel::new(context, prelude, model_src, "compute_auxiliary_fields", &model_bindings),
            wall_functions: Kernel::new(context, prelude, model_src, "compute_wall_functions", &model_bindings),
            transport: Kernel::new(context, prelude, model_src, "transport_iteration", &model_bindings),
            eddy_viscosity: Kernel::new(context, prelude, model_src, "compute_eddy_viscosity", &model_bindings),
            ghost_cells: Kernel::new(
                context, prelude, GHOST_CELLS_SRC, "main",
                &[Uniform, Uniform, ReadOnly, ReadOnly, ReadWrite]
            ),
            mirror_compute: Kernel::new(context, prelude, MIRROR_SRC, "compute", &mirror_bindings),
            mirror_scatter: Kernel::new(context, prelude, MIRROR_SRC, "scatter", &mirror_bindings),
            fixed_values: Kernel::new(
                context, prelude, FIXED_VALUES_SRC, "main", &[ReadOnly, ReadOnly, ReadWrite]
            ),
            damping: Kernel::new(context, prelude, DAMPING_SRC, "main", &[ReadOnly, ReadWrite]),
        }
    }
}

/// Bind groups for the kernels that are applied to both `fields` and `fields_next`
struct TargetBindGroups {
    /// One bind group per boundary face, in the order they must be applied, together with the
    /// workgroup counts for the face
    ghost_cells: Vec<(wgpu::BindGroup, [u32; 3])>,
    /// Bind groups for the compute and scatter phase
    mirror: (wgpu::BindGroup, wgpu::BindGroup),
    fixed_values: wgpu::BindGroup,
}

/// Bind groups for the model kernels, which all share the same buffers
struct ModelBindGroups {
    auxiliary_fields: wgpu::BindGroup,
    wall_functions: wgpu::BindGroup,
    transport: wgpu::BindGroup,
    eddy_viscosity: wgpu::BindGroup,
}

/// Turbulence solver executed on the GPU. See the module documentation for details.
pub struct TurbulenceSolverGPU {
    pub setup: TurbulenceSolverSetup,
    context: GpuContext,
    nr_extended_cells: usize,
    nr_fields: usize,
    kernels: TurbulenceKernels,

    params: TurbulenceParams,
    params_buffer: wgpu::Buffer,

    fields_buffer: wgpu::Buffer,
    fields_old_buffer: wgpu::Buffer,
    fields_next_buffer: wgpu::Buffer,

    interior_workgroups: [u32; 3],
    model_bind_groups: ModelBindGroups,
    target_bind_groups: [TargetBindGroups; 2],
    eddy_viscosity_ghost_cells: Vec<(wgpu::BindGroup, [u32; 3])>,
    damping_bind_group: wgpu::BindGroup,

    nr_mirror_entries: usize,
    nr_wall_function_entries: usize,
    nr_damping_entries: usize,
}

impl TurbulenceSolverGPU {
    /// Creates the solver on the device in `context`, which must be the device of the velocity
    /// solver. `velocity_buffer` and `eddy_viscosity_buffer` are the buffers of the velocity
    /// solver, which the turbulence solver reads from and writes to directly.
    pub fn new(
        context: GpuContext,
        grid: &Grid,
        setup: TurbulenceSolverSetup,
        velocity_buffer: wgpu::Buffer,
        eddy_viscosity_buffer: wgpu::Buffer,
    ) -> Self {
        let nr_extended_cells = grid.nr_extended_cells();
        let nr_fields = setup.model.nr_fields();
        let nr_auxiliary_fields = setup.model.nr_auxiliary_fields();

        assert!(
            nr_fields.max(nr_auxiliary_fields) * nr_extended_cells < u32::MAX as usize,
            "The grid is too large to be indexed with u32 values on the GPU"
        );

        let boundary_conditions = &setup.boundary_conditions;

        let prelude = format!(
            "{}\
             const UP_AXIS: u32 = {}u;\n\
             const N_EXTENDED_CELLS: u32 = {}u;\n\
             const NR_FIELDS: u32 = {}u;\n\
             const WALL_FUNCTION_FIELD: u32 = {}u;\n\
             const CONVECTION_SCHEME: u32 = {}u;\n\
             {}",
            common_constants_wgsl(),
            boundary_conditions.up_axis,
            nr_extended_cells,
            nr_fields,
            setup.model.wall_function_field(),
            setup.convection_scheme.as_gpu_flag(),
            COMMON_SRC
        );

        let model_src = format!(
            "{}\n{}\n{}",
            MODEL_BINDINGS_SRC,
            TRANSPORT_SRC,
            setup.model.wgsl_source()
        );

        let kernels = TurbulenceKernels::new(&context, &prelude, &model_src);

        let grid_buffer = grid.as_gpu_version().as_buffer(&context);

        // The time step is set before every update
        let params = TurbulenceParams::new(1.0, &setup);
        let params_buffer = context.create_uniform_buffer_init(&params);

        let initial_fields = boundary_conditions.initial_fields(grid);

        let fields_buffer = context.create_buffer_from_src(&initial_fields);
        let fields_old_buffer = context.create_buffer_from_src(&initial_fields);
        let fields_next_buffer = context.create_buffer_from_src(&initial_fields);
        let auxiliary_buffer = context.create_zeroed_buffer(nr_auxiliary_fields * nr_extended_cells);

        // --- Wall treatment ---
        // Empty buffers are not allowed as bindings, so a single zeroed entry is used instead when
        // there are no entries. The corresponding dispatches are skipped in that case.
        let (mirror_entries, wall_function_entries, damping_entries) = entries_for_gpu(&setup.wall_treatment);

        let mirror_entries_buffer = create_entries_buffer(&context, &mirror_entries);
        let mirror_values_buffer = context.create_zeroed_buffer((mirror_entries.len() * nr_fields).max(1));
        let wall_entries_buffer = create_entries_buffer(&context, &wall_function_entries);
        let wall_values_buffer = context.create_zeroed_buffer(wall_function_entries.len().max(1));
        let damping_entries_buffer = create_entries_buffer(&context, &damping_entries);

        let model_buffers = [
            &grid_buffer,
            &params_buffer,
            &velocity_buffer,
            &eddy_viscosity_buffer,
            &fields_buffer,
            &fields_old_buffer,
            &fields_next_buffer,
            &auxiliary_buffer,
            &wall_entries_buffer,
            &wall_values_buffer,
        ];

        let model_bind_groups = ModelBindGroups {
            auxiliary_fields: kernels.auxiliary_fields.bind_group(&context, &model_buffers),
            wall_functions: kernels.wall_functions.bind_group(&context, &model_buffers),
            transport: kernels.transport.bind_group(&context, &model_buffers),
            eddy_viscosity: kernels.eddy_viscosity.bind_group(&context, &model_buffers),
        };

        // --- Boundary conditions ---
        let inlet_profile_buffer = context.create_buffer_from_src(&boundary_conditions.inlet_profile);

        let ghost_cell_bind_groups = |field_buffer: &wgpu::Buffer, conditions: &[[ScalarBoundaryCondition; 2]; 3], nr_fields: usize| {
            let mut bind_groups = Vec::with_capacity(6);

            for axis_index in 0..3 {
                let (outer_axis, inner_axis) = match axis_index {
                    0 => (1, 2),
                    1 => (0, 2),
                    _ => (0, 1),
                };

                for face_index in 0..2 {
                    let face_params = GpuScalarFaceParams {
                        axis: axis_index as u32,
                        face: face_index as u32,
                        condition: conditions[axis_index][face_index].as_gpu_flag(),
                        nr_fields: nr_fields as u32,
                    };

                    let face_params_buffer = context.create_uniform_buffer_init(&face_params);

                    bind_groups.push((
                        kernels.ghost_cells.bind_group(
                            &context,
                            &[&grid_buffer, &face_params_buffer, &inlet_profile_buffer, &velocity_buffer, field_buffer]
                        ),
                        dispatch_plane(grid.extended_shape[inner_axis], grid.extended_shape[outer_axis])
                    ));
                }
            }

            bind_groups
        };

        let target_bind_groups = [&fields_buffer, &fields_next_buffer].map(|field_buffer| {
            let mirror_buffers = [&grid_buffer, &mirror_entries_buffer, field_buffer, &mirror_values_buffer];

            TargetBindGroups {
                ghost_cells: ghost_cell_bind_groups(field_buffer, &boundary_conditions.face_conditions, nr_fields),
                mirror: (
                    kernels.mirror_compute.bind_group(&context, &mirror_buffers),
                    kernels.mirror_scatter.bind_group(&context, &mirror_buffers),
                ),
                fixed_values: kernels.fixed_values.bind_group(
                    &context, &[&wall_entries_buffer, &wall_values_buffer, field_buffer]
                ),
            }
        });

        let eddy_viscosity_ghost_cells = ghost_cell_bind_groups(
            &eddy_viscosity_buffer, &[[ScalarBoundaryCondition::ZeroGradient; 2]; 3], 1
        );

        let damping_bind_group = kernels.damping.bind_group(
            &context, &[&damping_entries_buffer, &eddy_viscosity_buffer]
        );

        Self {
            nr_extended_cells,
            nr_fields,
            kernels,
            params,
            params_buffer,
            fields_buffer,
            fields_old_buffer,
            fields_next_buffer,
            interior_workgroups: dispatch_interior(grid.interior_shape),
            model_bind_groups,
            target_bind_groups,
            eddy_viscosity_ghost_cells,
            damping_bind_group,
            nr_mirror_entries: mirror_entries.len(),
            nr_wall_function_entries: wall_function_entries.len(),
            nr_damping_entries: damping_entries.len(),
            setup,
            context,
        }
    }

    /// See `TurbulenceSolverCPU::initialize`
    pub fn initialize(&mut self) {
        let mut encoder = self.create_encoder();

        self.record_mirror_corrections(&mut encoder, Target::Fields);
        self.record_ghost_cells(&mut encoder, Target::Fields);
        self.record_eddy_viscosity(&mut encoder);

        self.context.queue.submit([encoder.finish()]);
    }

    /// See `TurbulenceSolverCPU::update`
    pub fn update(&mut self, time_step: Float) {
        self.set_time_step(time_step);

        let fields_size = GpuContext::byte_length_from_length(self.nr_fields * self.nr_extended_cells);

        let mut encoder = self.create_encoder();

        encoder.copy_buffer_to_buffer(&self.fields_buffer, 0, &self.fields_old_buffer, 0, fields_size);

        self.kernels.auxiliary_fields.record(
            &mut encoder, &self.model_bind_groups.auxiliary_fields, self.interior_workgroups
        );

        if self.nr_wall_function_entries > 0 {
            self.kernels.wall_functions.record(
                &mut encoder,
                &self.model_bind_groups.wall_functions,
                dispatch_1d(self.nr_wall_function_entries)
            );
        }

        self.record_wall_function_values(&mut encoder, Target::Fields);

        for _ in 0..self.setup.nr_jacobi_iterations {
            self.kernels.transport.record(
                &mut encoder, &self.model_bind_groups.transport, self.interior_workgroups
            );

            self.record_wall_function_values(&mut encoder, Target::FieldsNext);
            self.record_mirror_corrections(&mut encoder, Target::FieldsNext);
            self.record_ghost_cells(&mut encoder, Target::FieldsNext);

            encoder.copy_buffer_to_buffer(&self.fields_next_buffer, 0, &self.fields_buffer, 0, fields_size);
        }

        self.record_eddy_viscosity(&mut encoder);

        self.context.queue.submit([encoder.finish()]);
    }

    /// Reads all the transported fields from the device
    pub fn read_fields(&self) -> Vec<Float> {
        self.context.read_buffer(&self.fields_buffer, self.nr_fields * self.nr_extended_cells)
    }

    fn create_encoder(&self) -> wgpu::CommandEncoder {
        self.context.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default())
    }

    /// Updates the parameter buffer, if the time step has changed
    fn set_time_step(&mut self, time_step: Float) {
        let params = TurbulenceParams::new(time_step, &self.setup);

        if params != self.params {
            self.params = params;

            self.context.write_pod_buffer(&self.params_buffer, &[params]);
        }
    }

    fn record_wall_function_values(&self, encoder: &mut wgpu::CommandEncoder, target: Target) {
        if self.nr_wall_function_entries > 0 {
            self.kernels.fixed_values.record(
                encoder,
                &self.target_bind_groups[target as usize].fixed_values,
                dispatch_1d(self.nr_wall_function_entries)
            );
        }
    }

    fn record_mirror_corrections(&self, encoder: &mut wgpu::CommandEncoder, target: Target) {
        if self.nr_mirror_entries > 0 {
            let (compute_bind_group, scatter_bind_group) = &self.target_bind_groups[target as usize].mirror;
            let workgroups = dispatch_1d(self.nr_mirror_entries);

            self.kernels.mirror_compute.record(encoder, compute_bind_group, workgroups);
            self.kernels.mirror_scatter.record(encoder, scatter_bind_group, workgroups);
        }
    }

    fn record_ghost_cells(&self, encoder: &mut wgpu::CommandEncoder, target: Target) {
        for (bind_group, workgroups) in &self.target_bind_groups[target as usize].ghost_cells {
            self.kernels.ghost_cells.record(encoder, bind_group, *workgroups);
        }
    }

    /// Computes the eddy viscosity from `fields`, damps it inside the no-slip geometries, and sets
    /// the ghost cells
    fn record_eddy_viscosity(&self, encoder: &mut wgpu::CommandEncoder) {
        self.kernels.eddy_viscosity.record(
            encoder, &self.model_bind_groups.eddy_viscosity, self.interior_workgroups
        );

        if self.nr_damping_entries > 0 {
            self.kernels.damping.record(encoder, &self.damping_bind_group, dispatch_1d(self.nr_damping_entries));
        }

        for (bind_group, workgroups) in &self.eddy_viscosity_ghost_cells {
            self.kernels.ghost_cells.record(encoder, bind_group, *workgroups);
        }
    }
}

/// Creates a storage buffer with `entries`, or with a single zeroed entry if `entries` is empty
fn create_entries_buffer<T: bytemuck::Pod>(context: &GpuContext, entries: &[T]) -> wgpu::Buffer {
    if entries.is_empty() {
        context.create_storage_buffer_init(&[T::zeroed()])
    } else {
        context.create_storage_buffer_init(entries)
    }
}

fn entries_for_gpu(
    wall_treatment: &WallTreatmentEntries
) -> (Vec<GpuMirrorEntry>, Vec<GpuWallFunctionEntry>, Vec<GpuDampingEntry>) {
    let mirror = wall_treatment.mirror.iter().map(|entry| {
        let mut weights = [0.0; 6];

        for axis in 0..3 {
            weights[2 * axis..2 * axis + 2].copy_from_slice(&entry.stencil.weights[axis]);
        }

        GpuMirrorEntry {
            cell_index: entry.cell_index as u32,
            base_index: entry.stencil.base_index as u32,
            weights,
        }
    }).collect();

    let wall_functions = wall_treatment.wall_functions.iter().map(|entry| {
        let mut weights = [0.0; 18];
        let mut base_index = [0u32; 3];

        for component in 0..3 {
            let stencil = &entry.velocity_stencils[component];

            base_index[component] = stencil.base_index as u32;

            for axis in 0..3 {
                let offset = 6 * component + 2 * axis;

                weights[offset..offset + 2].copy_from_slice(&stencil.weights[axis]);
            }
        }

        GpuWallFunctionEntry {
            cell_index: entry.cell_index as u32,
            wall_distance: entry.wall_distance,
            reference_distance: entry.reference_distance,
            normal: [entry.normal[0], entry.normal[1], entry.normal[2]],
            base_index,
            weights,
        }
    }).collect();

    let damping = wall_treatment.damping.iter().map(|entry| GpuDampingEntry {
        cell_index: entry.cell_index as u32,
        mu: entry.mu,
    }).collect();

    (mirror, wall_functions, damping)
}
