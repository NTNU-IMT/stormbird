pub mod boundary_condisitions;
pub mod kernels;
pub mod slip_mirror_stencils;
pub mod no_slip_corrections;
pub mod cpu;
pub mod gpu;

use std::borrow::Cow;

use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

use crate::grid::Grid;
use boundary_condisitions::VelocityBoundaryConditions;
use slip_mirror_stencils::SlipMirrorStencils;
use no_slip_corrections::NoSlipCorrections;

use cpu::VelocitySolverCPU;
use gpu::VelocitySolverGPU;

/// Everything the velocity solver needs that is computed once, when the simulation is built, and
/// that is independent of where the solver is executed. The CPU version uses this data directly,
/// while the GPU version uploads what it needs to the device.
pub struct VelocitySolverSetup {
    pub signed_distance_function: Vec<Float>,
    pub signed_distance_function_slip: Vec<Float>,
    pub normals_slip_surfaces: Vec<SpatialVector>,
    pub no_slip_corrections: NoSlipCorrections,
    pub slip_mirror_stencils: SlipMirrorStencils,
    pub boundary_conditions: VelocityBoundaryConditions,
    pub viscosity: Float,
    pub density: Float,
}

/// The velocity solver, executed either on the CPU or the GPU.
///
/// The methods here are the ones that are independent of where the pressure solver is executed.
/// The coupling to the pressure solver (computing the right hand side of the pressure equation
/// and adding the pressure gradient) depends on where both solvers live, and is therefore handled
/// by matching on the variants directly (see `Simulation::do_step`).
pub enum VelocitySolver {
    CPU(VelocitySolverCPU),
    GPU(VelocitySolverGPU),
}

impl VelocitySolver {
    pub fn setup(&self) -> &VelocitySolverSetup {
        match self {
            Self::CPU(solver) => &solver.setup,
            Self::GPU(solver) => &solver.setup,
        }
    }

    pub fn initialize_after_build(&mut self, grid: &Grid) {
        match self {
            Self::CPU(solver) => solver.initialize_after_build(grid),
            Self::GPU(solver) => solver.initialize_after_build(),
        }
    }

    pub fn initialize_before_step(&mut self, grid: &Grid) {
        match self {
            Self::CPU(solver) => solver.initialize_before_step(grid),
            Self::GPU(solver) => solver.initialize_before_step(),
        }
    }

    pub fn update_velocity_star(&mut self, grid: &Grid, time_step: Float) {
        match self {
            Self::CPU(solver) => solver.update_velocity_star(grid, time_step),
            Self::GPU(solver) => solver.update_velocity_star(time_step),
        }
    }

    /// Returns the largest magnitude of the (staggered) velocity vectors on the extended grid.
    /// Used to compute a time step from a Courant number.
    pub fn max_velocity(&self) -> Float {
        match self {
            Self::CPU(solver) => solver.max_velocity(),
            Self::GPU(solver) => solver.max_velocity(),
        }
    }

    /// Returns the cell-centered velocity at each of the cells in `cell_indices`, given as flat
    /// indices on the extended grid.
    pub fn cell_centered_velocity_at_cells(
        &mut self,
        grid: &Grid,
        cell_indices: &[usize]
    ) -> Vec<SpatialVector> {
        match self {
            Self::CPU(solver) => solver.cell_centered_velocity_at_cells(grid, cell_indices),
            Self::GPU(solver) => solver.cell_centered_velocity_at_cells(cell_indices),
        }
    }

    /// Sets the body force at each of the cells in `cell_indices`, given as flat indices on the
    /// extended grid. All other cells are left unchanged.
    pub fn set_body_force_at_cells(&mut self, cell_indices: &[usize], body_force: &[SpatialVector]) {
        match self {
            Self::CPU(solver) => solver.set_body_force_at_cells(cell_indices, body_force),
            Self::GPU(solver) => solver.set_body_force_at_cells(cell_indices, body_force),
        }
    }

    /// The velocity field on the host. Borrowed for the CPU version, and read from the device for
    /// the GPU version.
    pub fn velocity_host(&self) -> Cow<'_, [SpatialVector]> {
        match self {
            Self::CPU(solver) => Cow::Borrowed(&solver.velocity),
            Self::GPU(solver) => Cow::Owned(solver.read_velocity()),
        }
    }

    /// The body force field on the host. Borrowed for the CPU version, and read from the device
    /// for the GPU version.
    pub fn body_force_host(&self) -> Cow<'_, [SpatialVector]> {
        match self {
            Self::CPU(solver) => Cow::Borrowed(&solver.body_force),
            Self::GPU(solver) => Cow::Owned(solver.read_body_force()),
        }
    }
}
