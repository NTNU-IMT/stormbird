//! RANS turbulence models, solved separately from the velocity and the pressure.
//!
//! The coupling to the rest of the solver is deliberately minimal: the turbulence solver reads the
//! velocity, and writes the eddy viscosity, which the velocity solver uses for the turbulent
//! stresses (see `velocity_solver::kernels::turbulent_stress`). The turbulence solver is executed
//! once per time step, after the velocity and the pressure, and always on the same platform as the
//! velocity solver. On the GPU, the velocity and the eddy viscosity are shared directly between the
//! two solvers, so no data is transferred.
//!
//! Each time step consists of the following steps, which are the same for all models:
//!
//! 1. Copy the transported fields to the old fields.
//! 2. Compute the model's auxiliary fields (e.g., the production of k) from the velocity.
//! 3. Apply the wall functions, which modify the auxiliary fields and fix the wall-normal variable
//!    close to the no-slip walls.
//! 4. Solve the implicit transport equations with Jacobi iterations. After each iteration, the
//!    wall function values, the mirror (zero gradient) corrections inside the geometries, and the
//!    boundary conditions are applied.
//! 5. Compute the eddy viscosity, damp it inside the no-slip geometries, and set its ghost cells.

pub mod builder;
pub mod models;
pub mod boundary_conditions;
pub mod wall_treatment;
pub mod transport;
pub mod cpu;
pub mod gpu;

use std::borrow::Cow;

use stormath::type_aliases::Float;

use crate::grid::Grid;
use crate::velocity_solver::VelocitySolver;

use models::TurbulenceModel;
use boundary_conditions::TurbulenceBoundaryConditions;
use wall_treatment::{WallTreatmentEntries, WallFunctionConstants};
use transport::ConvectionScheme;

use cpu::TurbulenceSolverCPU;
use gpu::TurbulenceSolverGPU;

/// Everything the turbulence solver needs that is computed once, when the simulation is built, and
/// that is independent of where the solver is executed.
pub struct TurbulenceSolverSetup {
    pub model: TurbulenceModel,
    pub boundary_conditions: TurbulenceBoundaryConditions,
    pub wall_treatment: WallTreatmentEntries,
    pub wall_function_constants: WallFunctionConstants,
    /// The molecular viscosity
    pub viscosity: Float,
    pub convection_scheme: ConvectionScheme,
    pub nr_jacobi_iterations: usize,
    pub max_eddy_viscosity: Float,
}

/// The turbulence solver, executed on the same platform as the velocity solver
pub enum TurbulenceSolver {
    CPU(TurbulenceSolverCPU),
    GPU(TurbulenceSolverGPU),
}

const PLATFORM_MISMATCH: &str = "The turbulence solver must run on the same platform as the velocity solver";
const NO_EDDY_VISCOSITY: &str = "The velocity solver must be created with an eddy viscosity when a turbulence model is used";

impl TurbulenceSolver {
    pub fn setup(&self) -> &TurbulenceSolverSetup {
        match self {
            Self::CPU(solver) => &solver.setup,
            Self::GPU(solver) => &solver.setup,
        }
    }

    /// Applies the boundary conditions and the wall treatment to the initial fields, and computes
    /// the initial eddy viscosity. Must be called after the velocity solver is initialized.
    pub fn initialize(&mut self, grid: &Grid, velocity_solver: &mut VelocitySolver) {
        match (self, velocity_solver) {
            (Self::CPU(solver), VelocitySolver::CPU(velocity_solver)) => solver.initialize(
                grid,
                &velocity_solver.velocity,
                velocity_solver.eddy_viscosity.as_mut().expect(NO_EDDY_VISCOSITY)
            ),
            (Self::GPU(solver), VelocitySolver::GPU(_)) => solver.initialize(),
            _ => panic!("{}", PLATFORM_MISMATCH),
        }
    }

    /// Advances the turbulence model one time step, using the current velocity, and updates the
    /// eddy viscosity of the velocity solver.
    pub fn update(&mut self, grid: &Grid, velocity_solver: &mut VelocitySolver, time_step: Float) {
        match (self, velocity_solver) {
            (Self::CPU(solver), VelocitySolver::CPU(velocity_solver)) => solver.update(
                grid,
                &velocity_solver.velocity,
                velocity_solver.eddy_viscosity.as_mut().expect(NO_EDDY_VISCOSITY),
                time_step
            ),
            (Self::GPU(solver), VelocitySolver::GPU(_)) => solver.update(time_step),
            _ => panic!("{}", PLATFORM_MISMATCH),
        }
    }

    /// The names of the transported fields, in the order they are stored
    pub fn field_names(&self) -> &'static [&'static str] {
        self.setup().model.field_names()
    }

    /// All the transported fields on the host, stored field-major on the extended grid. Borrowed
    /// for the CPU version, and read from the device for the GPU version.
    pub fn fields_host(&self) -> Cow<'_, [Float]> {
        match self {
            Self::CPU(solver) => Cow::Borrowed(&solver.fields),
            Self::GPU(solver) => Cow::Owned(solver.read_fields()),
        }
    }
}
