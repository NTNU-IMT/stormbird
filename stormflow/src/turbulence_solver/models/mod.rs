//! The available turbulence models. Each model defines its transported fields, its auxiliary
//! fields, and the kernels that compute them, on both the CPU and the GPU. Everything else, such as
//! the boundary conditions, the wall treatment and the order of the steps, is shared by all models
//! (see `TurbulenceSolverCPU` and `TurbulenceSolverGPU`).
//!
//! To add a model:
//! 1. Add a module with the model coefficients and the CPU kernels, following
//!    `realizable_k_epsilon`.
//! 2. Add a variant to `TurbulenceModel`, and fill in the matches in this file and in the
//!    model-specific steps of `TurbulenceSolverCPU`.
//! 3. Add a shader with the same entry points as `realizable_k_epsilon.wgsl`, and return it from
//!    `TurbulenceModel::wgsl_source`.

pub mod realizable_k_epsilon;

use serde::{Serialize, Deserialize};

use stormath::type_aliases::Float;

use realizable_k_epsilon::RealizableKEpsilon;

#[derive(Debug, Clone, Serialize, Deserialize)]
/// The turbulence model, together with its coefficients
pub enum TurbulenceModel {
    RealizableKEpsilon(RealizableKEpsilon),
}

impl TurbulenceModel {
    /// The names of the transported fields, in the order they are stored
    pub fn field_names(&self) -> &'static [&'static str] {
        match self {
            Self::RealizableKEpsilon(_) => &RealizableKEpsilon::FIELD_NAMES,
        }
    }

    pub fn nr_fields(&self) -> usize {
        self.field_names().len()
    }

    /// The number of auxiliary fields, which are computed once per time step, before the transport
    /// equations are solved
    pub fn nr_auxiliary_fields(&self) -> usize {
        match self {
            Self::RealizableKEpsilon(_) => RealizableKEpsilon::NR_AUXILIARY_FIELDS,
        }
    }

    /// The index of the transported field that is fixed by the wall functions
    pub fn wall_function_field(&self) -> usize {
        match self {
            Self::RealizableKEpsilon(_) => realizable_k_epsilon::EPSILON,
        }
    }

    /// The values of the transported fields that correspond to the turbulent kinetic energy `k`
    /// and the dissipation rate `epsilon`. Used to set the inlet values, which are always specified
    /// through k and epsilon.
    pub fn fields_from_k_and_epsilon(&self, k: Float, epsilon: Float) -> Vec<Float> {
        match self {
            Self::RealizableKEpsilon(_) => vec![
                k.max(realizable_k_epsilon::K_MIN),
                epsilon.max(realizable_k_epsilon::EPSILON_MIN)
            ],
        }
    }

    /// The model specific constants, followed by the model shader, which must define the entry
    /// points `auxiliary`, `wall_functions`, `transport` and `eddy_viscosity`.
    pub fn wgsl_source(&self) -> String {
        match self {
            Self::RealizableKEpsilon(model) => format!(
                "{}\n{}",
                model.wgsl_constants(),
                include_str!("../gpu/shaders/realizable_k_epsilon.wgsl")
            ),
        }
    }
}
