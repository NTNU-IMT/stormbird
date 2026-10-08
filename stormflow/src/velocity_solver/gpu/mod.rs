//! GPU version of the velocity solver.
//!
//! All fields are stored on the device, as flat `f32` arrays with component `c` of cell `i` at
//! index `3 * i + c`. The kernels mirror the CPU version in [`super::cpu`] one-to-one, and use the
//! same precomputed data from [`VelocitySolverSetup`]. Data only moves between the host and the
//! device when explicitly requested: the max velocity, the velocity at the actuator line cells,
//! the body force at the same cells, the full fields for export, and the pressure coupling when the
//! pressure solver runs on the CPU.

mod kernels;

use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

use crate::gpu_interface::{
    context::GpuContext,
    utils as gpu_utils
};
use crate::grid::Grid;

use super::VelocitySolverSetup;
use super::no_slip_corrections::NoSlipCorrections;
use super::slip_mirror_stencils::{SlipMirrorStencils, SlipMirrorInterpolationStencil};
use super::wall_model::WallStressEntries;
use crate::log_law::NR_FRICTION_VELOCITY_ITERATIONS;

use kernels::{
    Kernel,
    Binding,
    ShaderConstants,
    dispatch_interior,
    dispatch_1d,
    dispatch_plane,
    dispatch_reduce,
};

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
/// Parameters that change with the time step. Matches `VelocityParams` in common.wgsl. The derived
/// values are computed in the same way as in the CPU version, so both versions use identical
/// coefficients.
struct VelocityParams {
    time_step: f32,
    viscosity: f32,
    inv_density: f32,
    rhs_scale: f32,
    gradient_scale: f32,
    _pad: [f32; 3],
}

impl VelocityParams {
    fn new(time_step: Float, viscosity: Float, density: Float) -> Self {
        let inv_density = 1.0 / density;
        let inv_time_step = 1.0 / time_step;

        Self {
            time_step,
            viscosity,
            inv_density,
            rhs_scale: density * inv_time_step,
            gradient_scale: time_step * inv_density,
            _pad: [0.0; 3],
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
/// Matches `NoSlipEntry` in no_slip_correction.wgsl
struct GpuNoSlipEntry {
    field_index: u32,
    mu: f32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
/// Matches `SlipEntry` in slip_correction.wgsl
struct GpuSlipEntry {
    field_index: u32,
    mu: f32,
    normal: [f32; 3],
    base_index: [u32; 3],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
/// Matches `WallStressEntry` in wall_stress.wgsl
struct GpuWallStressEntry {
    cell_index: u32,
    axis: u32,
    delta: f32,
    tangential_factor: f32,
    normal: [f32; 3],
    base_index: [u32; 3],
    weights: [f32; 18],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
/// Matches `FaceParams` in ghost_cells.wgsl
struct GpuFaceParams {
    axis: u32,
    face: u32,
    condition: u32,
    _pad: u32,
}

/// Device buffers owned by the pressure solver, which the velocity solver reads from and writes
/// to directly when both solvers run on the same GPU.
pub struct SharedPressureBuffers {
    /// The right hand side of the pressure equation, on the **interior** grid
    pub rhs: wgpu::Buffer,
    /// The pressure, on the **extended** grid
    pub pressure: wgpu::Buffer,
}

#[derive(Debug, Clone, Copy)]
/// The vector fields that the geometry corrections and boundary conditions are applied to
enum Field {
    Velocity = 0,
    VelocityStar = 1,
}

struct VelocityKernels {
    convect_and_diffuse: Kernel,
    pressure_rhs: Kernel,
    add_pressure_gradient: Kernel,
    no_slip_correction: Kernel,
    slip_compute: Kernel,
    slip_scatter: Kernel,
    ghost_cells: Kernel,
    max_velocity: Kernel,
    gather_cells: Kernel,
    scatter_cells: Kernel,
    /// `None` when the no-slip geometries use the data immersion
    wall_stress: Option<Kernel>,
    /// `None` when the velocity limiter is not used
    limit_velocity: Option<Kernel>,
}

impl VelocityKernels {
    /// `use_eddy_viscosity` selects the version of the convect and diffuse kernel that includes the
    /// turbulent stresses, which has the eddy viscosity as an additional binding.
    fn new(
        context: &GpuContext,
        constants: &ShaderConstants,
        use_eddy_viscosity: bool,
        wall_stress: &WallStressEntries,
        velocity_limit: Option<Float>,
    ) -> Self {
        use Binding::*;

        let prelude = constants.prelude();
        let constants = prelude.as_str();

        let slip_bindings = [Uniform, ReadOnly, ReadOnly, ReadWrite, ReadWrite];
        let cell_sampling_bindings = [Uniform, ReadOnly, ReadOnly, ReadWrite, ReadWrite];

        let convect_and_diffuse = if use_eddy_viscosity {
            Kernel::new(
                context,
                constants,
                &format!("{}
{}", kernels::CONVECT_AND_DIFFUSE_SRC, kernels::TURBULENT_STRESS_SRC),
                "main_turbulent",
                &[Uniform, Uniform, ReadOnly, ReadOnly, ReadOnly, ReadWrite, ReadOnly]
            )
        } else {
            Kernel::new(
                context, constants, kernels::CONVECT_AND_DIFFUSE_SRC, "main",
                &[Uniform, Uniform, ReadOnly, ReadOnly, ReadOnly, ReadWrite]
            )
        };

        Self {
            convect_and_diffuse,
            pressure_rhs: Kernel::new(
                context, constants, kernels::PRESSURE_RHS_SRC, "main",
                &[Uniform, Uniform, ReadOnly, ReadWrite]
            ),
            add_pressure_gradient: Kernel::new(
                context, constants, kernels::ADD_PRESSURE_GRADIENT_SRC, "main",
                &[Uniform, Uniform, ReadOnly, ReadOnly, ReadWrite]
            ),
            no_slip_correction: Kernel::new(
                context, constants, kernels::NO_SLIP_CORRECTION_SRC, "main",
                &[ReadOnly, ReadWrite]
            ),
            slip_compute: Kernel::new(
                context, constants, kernels::SLIP_CORRECTION_SRC, "compute", &slip_bindings
            ),
            slip_scatter: Kernel::new(
                context, constants, kernels::SLIP_CORRECTION_SRC, "scatter", &slip_bindings
            ),
            ghost_cells: Kernel::new(
                context, constants, kernels::GHOST_CELLS_SRC, "main",
                &[Uniform, Uniform, ReadOnly, ReadWrite]
            ),
            max_velocity: Kernel::new(
                context, constants, kernels::MAX_VELOCITY_SRC, "main",
                &[ReadOnly, ReadWrite]
            ),
            gather_cells: Kernel::new(
                context, constants, kernels::CELL_SAMPLING_SRC, "gather", &cell_sampling_bindings
            ),
            scatter_cells: Kernel::new(
                context, constants, kernels::CELL_SAMPLING_SRC, "scatter", &cell_sampling_bindings
            ),
            wall_stress: (!wall_stress.entries.is_empty()).then(|| {
                let log_law_constants = format!(
                    "const WALL_KAPPA: f32 = {:?};
                     const WALL_E: f32 = {:?};
                     const WALL_Y_PLUS_LAM: f32 = {:?};
                     const WALL_REFERENCE_DISTANCE: f32 = {:?};
                     const NR_FRICTION_VELOCITY_ITERATIONS: u32 = {}u;
",
                    wall_stress.constants.kappa,
                    wall_stress.constants.e,
                    wall_stress.constants.y_plus_lam(),
                    wall_stress.reference_distance,
                    NR_FRICTION_VELOCITY_ITERATIONS
                );

                Kernel::new(
                    context,
                    constants,
                    &format!("{}{}", log_law_constants, kernels::WALL_STRESS_SRC),
                    "main",
                    &[Uniform, Uniform, ReadOnly, ReadOnly, ReadWrite]
                )
            }),
            limit_velocity: velocity_limit.map(|limit| {
                Kernel::new(
                    context,
                    constants,
                    &format!("const VELOCITY_LIMIT: f32 = {:?};\n{}", limit, kernels::LIMIT_VELOCITY_SRC),
                    "main",
                    &[ReadWrite, ReadWrite]
                )
            }),
        }
    }
}

/// Bind groups for the kernels that are applied to both `velocity` and `velocity_star`
struct FieldBindGroups {
    /// `None` if there are no no-slip corrections
    no_slip: Option<wgpu::BindGroup>,
    /// Bind groups for the compute and scatter phase. `None` if there are no slip corrections
    slip: Option<(wgpu::BindGroup, wgpu::BindGroup)>,
    /// One bind group per boundary face, in the order they must be applied, together with the
    /// workgroup counts for the face
    ghost_cells: Vec<(wgpu::BindGroup, [u32; 3])>,
}

/// Buffers for exchanging data with the actuator line model, for a specific set of cells
struct CellSampling {
    cell_indices: Vec<usize>,
    cell_values_buffer: wgpu::Buffer,
    gather_bind_group: wgpu::BindGroup,
    scatter_bind_group: wgpu::BindGroup,
    workgroups: [u32; 3],
}

/// Velocity solver executed on the GPU. See the module documentation for details.
pub struct VelocitySolverGPU {
    pub setup: VelocitySolverSetup,
    context: GpuContext,
    nr_extended_cells: usize,
    nr_interior_cells: usize,
    kernels: VelocityKernels,

    grid_buffer: wgpu::Buffer,
    params: VelocityParams,
    params_buffer: wgpu::Buffer,

    velocity_buffer: wgpu::Buffer,
    velocity_org_buffer: wgpu::Buffer,
    velocity_star_buffer: wgpu::Buffer,
    body_force_buffer: wgpu::Buffer,
    /// The cell-centered eddy viscosity, set by the turbulence solver. `None` when no turbulence
    /// model is used.
    eddy_viscosity_buffer: Option<wgpu::Buffer>,
    /// Either owned by this solver, or shared with the pressure solver
    rhs_buffer: wgpu::Buffer,
    /// Either owned by this solver, or shared with the pressure solver
    pressure_buffer: wgpu::Buffer,

    interior_workgroups: [u32; 3],
    convect_and_diffuse_bind_group: wgpu::BindGroup,
    /// Together with the number of entries. `None` when the no-slip geometries use the data
    /// immersion.
    wall_stress_bind_group: Option<(wgpu::BindGroup, usize)>,
    pressure_rhs_bind_group: wgpu::BindGroup,
    add_pressure_gradient_bind_group: wgpu::BindGroup,

    nr_no_slip_entries: usize,
    nr_slip_entries: usize,
    field_bind_groups: [FieldBindGroups; 2],

    max_velocity_result_buffer: wgpu::Buffer,
    max_velocity_bind_group: wgpu::BindGroup,

    /// Counts the velocity components clipped by the velocity limiter during the current time
    /// step, as a u32
    limited_values_counter_buffer: wgpu::Buffer,
    /// Bind groups of the velocity limiter for `velocity` and `velocity_star`. `None` when the
    /// velocity limiter is not used.
    limit_velocity_bind_groups: Option<[wgpu::BindGroup; 2]>,
    max_velocity_workgroups: [u32; 3],

    /// Created on the first request for data at a set of cells, and recreated if the cells change
    cell_sampling: Option<CellSampling>,
}

impl VelocitySolverGPU {
    /// Creates the solver on the device in `context`. If the pressure solver runs on the same
    /// device, its buffers should be given in `shared_pressure_buffers`, so that the pressure
    /// coupling happens without any transfers. Otherwise, the solver allocates its own buffers,
    /// that must be synchronized with the pressure solver through `read_pressure_rhs` and
    /// `write_pressure`. If `use_eddy_viscosity` is true, an eddy viscosity buffer is allocated,
    /// initialized to zero, which the turbulence solver writes to directly (see
    /// `eddy_viscosity_buffer`).
    pub fn new(
        context: GpuContext,
        grid: &Grid,
        setup: VelocitySolverSetup,
        initial_velocity: &[SpatialVector],
        shared_pressure_buffers: Option<SharedPressureBuffers>,
        use_eddy_viscosity: bool
    ) -> Self {
        let nr_extended_cells = grid.nr_extended_cells();
        let nr_interior_cells = grid.nr_interior_cells();

        assert_eq!(initial_velocity.len(), nr_extended_cells);
        assert!(
            3 * nr_extended_cells < u32::MAX as usize,
            "The grid is too large to be indexed with u32 values on the GPU"
        );

        let boundary_conditions = &setup.boundary_conditions;
        let up_axis = boundary_conditions.up_axis;

        assert_eq!(
            boundary_conditions.inlet_velocity_profile.len(),
            grid.extended_shape[up_axis],
            "The inlet velocity profile does not match the grid"
        );

        let no_slip_entries = no_slip_entries_for_gpu(&setup.no_slip_corrections);
        let (slip_entries, slip_weights, slip_n) = slip_entries_for_gpu(&setup.slip_mirror_stencils);

        let constants = ShaderConstants {
            up_axis,
            slip_n,
            nr_extended_cells,
        };

        let kernels = VelocityKernels::new(
            &context, &constants, use_eddy_viscosity, &setup.wall_stress, setup.velocity_limit
        );

        let grid_buffer = grid.as_gpu_version().as_buffer(&context);

        // The time step is set before every use of the parameters
        let params = VelocityParams::new(1.0, setup.viscosity, setup.density);
        let params_buffer = context.create_uniform_buffer_init(&params);

        let initial_velocity_flat = gpu_utils::flatten_spatial_vectors(initial_velocity);

        let velocity_buffer = context.create_buffer_from_src(&initial_velocity_flat);
        let velocity_org_buffer = context.create_buffer_from_src(&initial_velocity_flat);
        let velocity_star_buffer = context.create_buffer_from_src(&initial_velocity_flat);
        let body_force_buffer = context.create_zeroed_buffer(3 * nr_extended_cells);
        let eddy_viscosity_buffer = use_eddy_viscosity.then(
            || context.create_zeroed_buffer(nr_extended_cells)
        );

        let (rhs_buffer, pressure_buffer) = match shared_pressure_buffers {
            Some(buffers) => (buffers.rhs, buffers.pressure),
            None => (
                context.create_zeroed_buffer(nr_interior_cells),
                context.create_zeroed_buffer(nr_extended_cells)
            )
        };

        let mut convect_and_diffuse_buffers = vec![
            &grid_buffer,
            &params_buffer,
            &velocity_org_buffer,
            &velocity_buffer,
            &body_force_buffer,
            &velocity_star_buffer
        ];

        if let Some(eddy_viscosity_buffer) = &eddy_viscosity_buffer {
            convect_and_diffuse_buffers.push(eddy_viscosity_buffer);
        }

        let convect_and_diffuse_bind_group = kernels.convect_and_diffuse.bind_group(
            &context, &convect_and_diffuse_buffers
        );

        let wall_stress_bind_group = kernels.wall_stress.as_ref().map(|kernel| {
            let entries = wall_stress_entries_for_gpu(&setup.wall_stress);
            let entries_buffer = context.create_storage_buffer_init(&entries);

            (
                kernel.bind_group(
                    &context,
                    &[&grid_buffer, &params_buffer, &entries_buffer, &velocity_buffer, &velocity_star_buffer]
                ),
                entries.len()
            )
        });

        let pressure_rhs_bind_group = kernels.pressure_rhs.bind_group(
            &context,
            &[&grid_buffer, &params_buffer, &velocity_star_buffer, &rhs_buffer]
        );

        let add_pressure_gradient_bind_group = kernels.add_pressure_gradient.bind_group(
            &context,
            &[&grid_buffer, &params_buffer, &pressure_buffer, &velocity_star_buffer, &velocity_buffer]
        );

        // --- Geometry corrections ---
        // Empty buffers are not allowed as bindings, so the corrections are skipped entirely when
        // there are no entries.
        let no_slip_entries_buffer = (!no_slip_entries.is_empty()).then(
            || context.create_storage_buffer_init(&no_slip_entries)
        );

        let slip_buffers = (!slip_entries.is_empty()).then(|| (
            context.create_storage_buffer_init(&slip_entries),
            context.create_storage_buffer_init(&slip_weights),
            context.create_zeroed_buffer(slip_entries.len()),
        ));

        // --- Boundary conditions ---
        let inlet_velocity_profile_buffer = context.create_storage_buffer_init(
            &gpu_utils::flatten_spatial_vectors(&boundary_conditions.inlet_velocity_profile)
        );

        let mut face_params_buffers: Vec<(wgpu::Buffer, [u32; 3])> = Vec::with_capacity(6);

        for axis_index in 0..3 {
            let (outer_axis, inner_axis) = match axis_index {
                0 => (1, 2),
                1 => (0, 2),
                _ => (0, 1),
            };

            for face_index in 0..2 {
                let face_params = GpuFaceParams {
                    axis: axis_index as u32,
                    face: face_index as u32,
                    condition: boundary_conditions.face_conditions[axis_index][face_index].as_gpu_flag(),
                    _pad: 0,
                };

                face_params_buffers.push((
                    context.create_uniform_buffer_init(&face_params),
                    dispatch_plane(grid.extended_shape[inner_axis], grid.extended_shape[outer_axis])
                ));
            }
        }

        let field_bind_groups = [&velocity_buffer, &velocity_star_buffer].map(|field_buffer| {
            FieldBindGroups {
                no_slip: no_slip_entries_buffer.as_ref().map(|entries_buffer| {
                    kernels.no_slip_correction.bind_group(&context, &[entries_buffer, field_buffer])
                }),
                slip: slip_buffers.as_ref().map(|(entries_buffer, weights_buffer, new_values_buffer)| {
                    let buffers = [
                        &grid_buffer, entries_buffer, weights_buffer, field_buffer, new_values_buffer
                    ];

                    (
                        kernels.slip_compute.bind_group(&context, &buffers),
                        kernels.slip_scatter.bind_group(&context, &buffers)
                    )
                }),
                ghost_cells: face_params_buffers.iter().map(|(face_params_buffer, workgroups)| {
                    (
                        kernels.ghost_cells.bind_group(
                            &context,
                            &[&grid_buffer, face_params_buffer, &inlet_velocity_profile_buffer, field_buffer]
                        ),
                        *workgroups
                    )
                }).collect(),
            }
        });

        // --- Velocity limiter ---
        let limited_values_counter_buffer = context.create_zeroed_buffer(1);

        let limit_velocity_bind_groups = kernels.limit_velocity.as_ref().map(|kernel| {
            [&velocity_buffer, &velocity_star_buffer].map(|field_buffer| {
                kernel.bind_group(&context, &[field_buffer, &limited_values_counter_buffer])
            })
        });

        // --- Max velocity ---
        let max_velocity_result_buffer = context.create_zeroed_buffer(1);

        let max_velocity_bind_group = kernels.max_velocity.bind_group(
            &context,
            &[&velocity_buffer, &max_velocity_result_buffer]
        );

        Self {
            setup,
            nr_extended_cells,
            nr_interior_cells,
            kernels,
            grid_buffer,
            params,
            params_buffer,
            velocity_buffer,
            velocity_org_buffer,
            velocity_star_buffer,
            body_force_buffer,
            eddy_viscosity_buffer,
            rhs_buffer,
            pressure_buffer,
            interior_workgroups: dispatch_interior(grid.interior_shape),
            convect_and_diffuse_bind_group,
            wall_stress_bind_group,
            pressure_rhs_bind_group,
            add_pressure_gradient_bind_group,
            nr_no_slip_entries: no_slip_entries.len(),
            nr_slip_entries: slip_entries.len(),
            field_bind_groups,
            max_velocity_result_buffer,
            max_velocity_bind_group,
            limited_values_counter_buffer,
            limit_velocity_bind_groups,
            max_velocity_workgroups: dispatch_reduce(nr_extended_cells),
            cell_sampling: None,
            context,
        }
    }

    pub fn initialize_after_build(&mut self) {
        let mut encoder = self.create_encoder();

        self.record_geometry_corrections(&mut encoder, Field::Velocity);
        self.record_geometry_corrections(&mut encoder, Field::VelocityStar);

        self.context.queue.submit([encoder.finish()]);
    }

    pub fn initialize_before_step(&mut self) {
        let mut encoder = self.create_encoder();

        self.record_ghost_cells(&mut encoder, Field::Velocity);

        encoder.clear_buffer(&self.limited_values_counter_buffer, 0, None);

        encoder.copy_buffer_to_buffer(
            &self.velocity_buffer, 0,
            &self.velocity_org_buffer, 0,
            GpuContext::byte_length_from_length(3 * self.nr_extended_cells)
        );

        self.context.queue.submit([encoder.finish()]);
    }

    pub fn update_velocity_star(&mut self, time_step: Float) {
        self.set_time_step(time_step);

        let mut encoder = self.create_encoder();

        self.kernels.convect_and_diffuse.record(
            &mut encoder, &self.convect_and_diffuse_bind_group, self.interior_workgroups
        );

        if let (Some(kernel), Some((bind_group, nr_entries))) = (&self.kernels.wall_stress, &self.wall_stress_bind_group) {
            kernel.record(&mut encoder, bind_group, dispatch_1d(*nr_entries));
        }

        self.record_velocity_limiter(&mut encoder, Field::VelocityStar);

        self.record_geometry_corrections(&mut encoder, Field::VelocityStar);
        self.record_ghost_cells(&mut encoder, Field::VelocityStar);

        self.context.queue.submit([encoder.finish()]);
    }

    /// Computes the right hand side of the pressure Poisson equation from `velocity_star`, and
    /// writes it to the rhs buffer on the device.
    pub fn compute_pressure_rhs(&mut self, time_step: Float) {
        self.set_time_step(time_step);

        let mut encoder = self.create_encoder();

        self.kernels.pressure_rhs.record(
            &mut encoder, &self.pressure_rhs_bind_group, self.interior_workgroups
        );

        self.context.queue.submit([encoder.finish()]);
    }

    /// Reads the right hand side of the pressure equation from the device into `rhs`, which is
    /// stored on the **interior** grid. Only needed when the pressure solver runs on the CPU.
    pub fn read_pressure_rhs(&self, rhs: &mut [Float]) {
        rhs.copy_from_slice(&self.context.read_buffer(&self.rhs_buffer, self.nr_interior_cells));
    }

    /// Writes `pressure`, which is stored on the **extended** grid, to the pressure buffer on the
    /// device. Only needed when the pressure solver runs on the CPU.
    pub fn write_pressure(&self, pressure: &[Float]) {
        assert_eq!(pressure.len(), self.nr_extended_cells);

        self.context.write_buffer(&self.pressure_buffer, pressure);
    }

    /// Adds the gradient of the pressure in the pressure buffer on the device to `velocity_star`
    pub fn update_velocity(&mut self, time_step: Float) {
        self.set_time_step(time_step);

        let mut encoder = self.create_encoder();

        self.kernels.add_pressure_gradient.record(
            &mut encoder, &self.add_pressure_gradient_bind_group, self.interior_workgroups
        );

        self.record_velocity_limiter(&mut encoder, Field::Velocity);

        self.record_geometry_corrections(&mut encoder, Field::Velocity);
        self.record_ghost_cells(&mut encoder, Field::Velocity);

        self.context.queue.submit([encoder.finish()]);
    }

    /// Returns the largest magnitude of the (staggered) velocity vectors on the extended grid.
    /// Only a single value is transferred from the device.
    pub fn max_velocity(&self) -> Float {
        let mut encoder = self.create_encoder();

        encoder.clear_buffer(&self.max_velocity_result_buffer, 0, None);

        self.kernels.max_velocity.record(
            &mut encoder, &self.max_velocity_bind_group, self.max_velocity_workgroups
        );

        self.context.queue.submit([encoder.finish()]);

        // The result is stored as the bit pattern of the f32 value
        self.context.read_buffer(&self.max_velocity_result_buffer, 1)[0]
    }

    /// Returns the cell-centered velocity at each of the cells in `cell_indices`, given as flat
    /// indices on the extended grid. Only the values at these cells are transferred from the
    /// device.
    pub fn cell_centered_velocity_at_cells(&mut self, cell_indices: &[usize]) -> Vec<SpatialVector> {
        if cell_indices.is_empty() {
            return Vec::new();
        }

        self.prepare_cell_sampling(cell_indices);

        let cell_sampling = self.cell_sampling.as_ref().unwrap();

        let mut encoder = self.create_encoder();

        self.kernels.gather_cells.record(
            &mut encoder, &cell_sampling.gather_bind_group, cell_sampling.workgroups
        );

        self.context.queue.submit([encoder.finish()]);

        let values = self.context.read_buffer(
            &cell_sampling.cell_values_buffer,
            3 * cell_indices.len()
        );

        gpu_utils::unflatten_spatial_vectors(&values)
    }

    /// Sets the body force at each of the cells in `cell_indices`, given as flat indices on the
    /// extended grid. All other cells are left unchanged. Only the values at these cells are
    /// transferred to the device.
    pub fn set_body_force_at_cells(&mut self, cell_indices: &[usize], body_force: &[SpatialVector]) {
        assert_eq!(cell_indices.len(), body_force.len());

        if cell_indices.is_empty() {
            return;
        }

        self.prepare_cell_sampling(cell_indices);

        let cell_sampling = self.cell_sampling.as_ref().unwrap();

        self.context.write_buffer(
            &cell_sampling.cell_values_buffer,
            &gpu_utils::flatten_spatial_vectors(body_force)
        );

        let mut encoder = self.create_encoder();

        self.kernels.scatter_cells.record(
            &mut encoder, &cell_sampling.scatter_bind_group, cell_sampling.workgroups
        );

        self.context.queue.submit([encoder.finish()]);
    }

    /// Reads the full velocity field from the device
    pub fn read_velocity(&self) -> Vec<SpatialVector> {
        self.read_vector_field(&self.velocity_buffer)
    }

    /// Reads the full `velocity_star` field from the device
    pub fn read_velocity_star(&self) -> Vec<SpatialVector> {
        self.read_vector_field(&self.velocity_star_buffer)
    }

    /// Reads the full body force field from the device
    pub fn read_body_force(&self) -> Vec<SpatialVector> {
        self.read_vector_field(&self.body_force_buffer)
    }

    /// Reads the full eddy viscosity field from the device, if it exists
    pub fn read_eddy_viscosity(&self) -> Option<Vec<Float>> {
        self.eddy_viscosity_buffer.as_ref().map(
            |buffer| self.context.read_buffer(buffer, self.nr_extended_cells)
        )
    }

    /// The velocity buffer on the device, for solvers on the same device that read the velocity
    /// directly, such as the turbulence solver.
    pub fn velocity_buffer(&self) -> &wgpu::Buffer {
        &self.velocity_buffer
    }

    /// The eddy viscosity buffer on the device, which the turbulence solver writes to directly.
    /// `None` if the solver was created without an eddy viscosity.
    pub fn eddy_viscosity_buffer(&self) -> Option<&wgpu::Buffer> {
        self.eddy_viscosity_buffer.as_ref()
    }

    /// The device context of the solver
    pub fn context(&self) -> &GpuContext {
        &self.context
    }

    fn read_vector_field(&self, buffer: &wgpu::Buffer) -> Vec<SpatialVector> {
        gpu_utils::unflatten_spatial_vectors(
            &self.context.read_buffer(buffer, 3 * self.nr_extended_cells)
        )
    }

    fn create_encoder(&self) -> wgpu::CommandEncoder {
        self.context.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default())
    }

    /// Updates the parameter buffer, if the time step has changed. The write is executed before
    /// any work submitted after this call.
    fn set_time_step(&mut self, time_step: Float) {
        let params = VelocityParams::new(time_step, self.setup.viscosity, self.setup.density);

        if params != self.params {
            self.params = params;

            self.context.write_pod_buffer(&self.params_buffer, &[params]);
        }
    }

    /// Records the velocity limiter on `field`, if the limiter is used
    fn record_velocity_limiter(&self, encoder: &mut wgpu::CommandEncoder, field: Field) {
        if let (Some(kernel), Some(bind_groups)) = (&self.kernels.limit_velocity, &self.limit_velocity_bind_groups) {
            kernel.record(encoder, &bind_groups[field as usize], dispatch_1d(3 * self.nr_extended_cells));
        }
    }

    /// The number of velocity components clipped by the velocity limiter during the last time
    /// step. Only a single value is transferred from the device.
    pub fn nr_limited_velocity_values(&self) -> usize {
        if self.limit_velocity_bind_groups.is_none() {
            return 0;
        }

        // The counter is stored as a u32, and read as the bit pattern of an f32
        self.context.read_buffer(&self.limited_values_counter_buffer, 1)[0].to_bits() as usize
    }

    /// Records first the no-slip and then the slip geometry corrections of `field`
    fn record_geometry_corrections(&self, encoder: &mut wgpu::CommandEncoder, field: Field) {
        let bind_groups = &self.field_bind_groups[field as usize];

        if let Some(bind_group) = &bind_groups.no_slip {
            self.kernels.no_slip_correction.record(
                encoder, bind_group, dispatch_1d(self.nr_no_slip_entries)
            );
        }

        if let Some((compute_bind_group, scatter_bind_group)) = &bind_groups.slip {
            let workgroups = dispatch_1d(self.nr_slip_entries);

            self.kernels.slip_compute.record(encoder, compute_bind_group, workgroups);
            self.kernels.slip_scatter.record(encoder, scatter_bind_group, workgroups);
        }
    }

    /// Records the ghost cell update of `field`, one boundary face at a time
    fn record_ghost_cells(&self, encoder: &mut wgpu::CommandEncoder, field: Field) {
        for (bind_group, workgroups) in &self.field_bind_groups[field as usize].ghost_cells {
            self.kernels.ghost_cells.record(encoder, bind_group, *workgroups);
        }
    }

    /// Makes sure `self.cell_sampling` matches `cell_indices`. The buffers are only recreated if
    /// the cells change, which normally never happens after the first time step.
    fn prepare_cell_sampling(&mut self, cell_indices: &[usize]) {
        if let Some(cell_sampling) = &self.cell_sampling &&
            cell_sampling.cell_indices == cell_indices {
            return;
        }

        let cell_indices_u32: Vec<u32> = cell_indices.iter().map(|&i| {
            assert!(i < self.nr_extended_cells, "Cell index {} is outside the grid", i);

            i as u32
        }).collect();

        let cell_indices_buffer = self.context.create_storage_buffer_init(&cell_indices_u32);
        let cell_values_buffer = self.context.create_zeroed_buffer(3 * cell_indices.len());

        let buffers = [
            &self.grid_buffer,
            &cell_indices_buffer,
            &self.velocity_buffer,
            &self.body_force_buffer,
            &cell_values_buffer
        ];

        self.cell_sampling = Some(CellSampling {
            cell_indices: cell_indices.to_vec(),
            gather_bind_group: self.kernels.gather_cells.bind_group(&self.context, &buffers),
            scatter_bind_group: self.kernels.scatter_cells.bind_group(&self.context, &buffers),
            cell_values_buffer,
            workgroups: dispatch_1d(cell_indices.len()),
        });
    }
}

/// Flattens the no-slip corrections for all three axes into one list, where each entry targets
/// one component of one cell.
fn no_slip_entries_for_gpu(corrections: &NoSlipCorrections) -> Vec<GpuNoSlipEntry> {
    let mut entries = Vec::new();

    for axis_index in 0..3 {
        for entry in &corrections.entries[axis_index] {
            entries.push(GpuNoSlipEntry {
                field_index: (3 * entry.cell_index + axis_index) as u32,
                mu: entry.mu,
            });
        }
    }

    entries
}

fn wall_stress_entries_for_gpu(wall_stress: &WallStressEntries) -> Vec<GpuWallStressEntry> {
    wall_stress.entries.iter().map(|entry| {
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

        GpuWallStressEntry {
            cell_index: entry.cell_index as u32,
            axis: entry.axis as u32,
            delta: entry.delta,
            tangential_factor: entry.tangential_factor,
            normal: [entry.normal[0], entry.normal[1], entry.normal[2]],
            base_index,
            weights,
        }
    }).collect()
}

/// Flattens the slip corrections for all three axes into one list, plus a separate list of the
/// interpolation weights (see `weights` in slip_correction.wgsl). Also returns the number of
/// weights per axis, which is the same for all entries.
fn slip_entries_for_gpu(stencils: &SlipMirrorStencils) -> (Vec<GpuSlipEntry>, Vec<Float>, usize) {
    let mut entries = Vec::new();
    let mut weights: Vec<Float> = Vec::new();

    // Only used to compile the shader if there are no entries
    let mut slip_n = 4;

    for axis_index in 0..3 {
        for entry in &stencils.entries[axis_index] {
            let mut base_index = [0u32; 3];

            for component in 0..3 {
                match &entry.component_stencils[component] {
                    SlipMirrorInterpolationStencil::Trilinear(stencil) => {
                        slip_n = 2;
                        base_index[component] = stencil.base_index as u32;

                        for axis_weights in &stencil.weights {
                            weights.extend_from_slice(axis_weights);
                        }
                    },
                    SlipMirrorInterpolationStencil::Tricubic(stencil) => {
                        slip_n = 4;
                        base_index[component] = stencil.base_index as u32;

                        for axis_weights in &stencil.weights {
                            weights.extend_from_slice(axis_weights);
                        }
                    },
                }
            }

            entries.push(GpuSlipEntry {
                field_index: (3 * entry.cell_index + axis_index) as u32,
                mu: entry.mu,
                normal: [entry.normal[0], entry.normal[1], entry.normal[2]],
                base_index,
            });
        }
    }

    assert_eq!(
        weights.len(),
        entries.len() * 9 * slip_n,
        "All slip mirror stencils must use the same interpolation order"
    );

    (entries, weights, slip_n)
}
