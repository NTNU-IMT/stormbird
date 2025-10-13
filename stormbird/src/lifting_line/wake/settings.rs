use serde::{Serialize, Deserialize};

use stormath::type_aliases::Float;

use crate::lifting_line::singularity_elements::symmetry_condition::SymmetryCondition;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Enum to represent different ways a viscous core length can be specified.
pub enum ViscousCoreLength {
    /// Signifies that the viscous core length is a fraction of the length of the vortex line. To
    /// be used, the vortex line geometry must be known.
    Relative(Float),
    /// Signifies that the viscous core length is an absolute value, and that it can be used without
    /// any more information about the geometry.
    Absolute(Float),
    /// Signifies that the viscous core length is not used.
    NoViscousCore,
}

impl Default for ViscousCoreLength {
    fn default() -> Self {
        Self::Relative(0.1)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Variables used whn constructing a quasi-steady wake model.
pub struct QuasiSteadyWakeSettings {
    #[serde(default="QuasiSteadyWakeSettings::default_wake_length_factor")]
    pub wake_length_factor: Float,
    #[serde(default)]
    pub symmetry_condition: SymmetryCondition,
    #[serde(default)]
    pub viscous_core_length: ViscousCoreLength,
}

impl QuasiSteadyWakeSettings {
    pub fn default_wake_length_factor() -> Float {100.0}
}

impl Default for QuasiSteadyWakeSettings {
    fn default() -> Self {
        Self {
            wake_length_factor: Self::default_wake_length_factor(),
            symmetry_condition: Default::default(),
            viscous_core_length: Default::default(),
        }
    }
}