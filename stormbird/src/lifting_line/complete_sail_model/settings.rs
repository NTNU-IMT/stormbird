

use serde::{Serialize, Deserialize};

use stormath::type_aliases::Float;
use stormath::spatial_vector::SpatialVector;

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Settings for the complete sail model that is unique to that model. That is, settings that are
/// not natural to put in either the lifting line simulation, wind environment, or the control system
pub struct CompleteSailModelSettings {
    #[serde(default="CompleteSailModelSettings::default_max_controller_iterations")]
    /// The maximum number of times the sail model will run a simulation when the goal is to adjust 
    /// control settings that depends on the solution. Most important case is to be able to adjust 
    /// the angle of attack based on lift-induced velocities
    pub max_controller_iterations: usize,
    #[serde(default="CompleteSailModelSettings::default_allowed_angle_error")]
    /// When searching for an angle of attack, it is typically OK with some error relative to the 
    /// perfectly converged case. This limits controls when to stop iterating
    pub allowed_angle_error: Float,
    #[serde(default="CompleteSailModelSettings::default_allowed_relative_internal_state_error")]
    /// The allowed relative change in the internal state of each wing, from the controller, when
    /// searching for a converged steady state. The model stops iterating when both the local wing
    /// angles and the internal states have converged.
    pub allowed_relative_internal_state_error: Float,
    #[serde(default="CompleteSailModelSettings::default_nr_loadings_to_test_during_optimization")]
    /// The number of loadings to test when the goal is to find the one with the highest delivered power
    pub nr_loadings_to_test_during_optimization: usize,
    /// Switch to determine whether the sails are retractable or not.
    #[serde(default)]
    pub retractable: bool,
    /// A vector defining the direction of the thrust
    #[serde(default="CompleteSailModelSettings::default_thrust_direction")]
    pub thrust_direction: SpatialVector
}

impl Default for CompleteSailModelSettings {
    fn default() -> Self {
        CompleteSailModelSettings{
            max_controller_iterations: Self::default_max_controller_iterations(),
            allowed_angle_error: Self::default_allowed_angle_error(),
            allowed_relative_internal_state_error: Self::default_allowed_relative_internal_state_error(),
            nr_loadings_to_test_during_optimization: Self::default_nr_loadings_to_test_during_optimization(),
            retractable: false,
            thrust_direction: Self::default_thrust_direction()
        }
    }
}

impl CompleteSailModelSettings {
    fn default_max_controller_iterations() -> usize {10}
    fn default_allowed_angle_error() -> Float {0.1_f64.to_radians() as Float}
    fn default_allowed_relative_internal_state_error() -> Float {1e-3}
    fn default_nr_loadings_to_test_during_optimization() -> usize {10}
    fn default_thrust_direction() -> SpatialVector {SpatialVector::new(-1.0, 0.0, 0.0)}
}
