use serde::{Deserialize, Serialize};

use super::prelude::*;


#[derive(Debug, Default, Clone, Serialize, Deserialize)]
/// A sail controller that attempts to set the wing angles to achieve a target effective angle of 
/// attack. 
pub struct EffectiveAngleOfAttackController {
    /// The set point for the effective angle of attack
    pub target_angles_of_attack: Vec<f64>,
}

impl EffectiveAngleOfAttackController {
    pub fn get_new_output(
        &self,
        angle_measurements: &[f64]
    ) -> ControllerOutput {
        let nr_of_wings = angle_measurements.len();

        let mut new_local_wing_angles = vec![0.0; nr_of_wings];

        for i in 0..nr_of_wings {
            let mut angle_error = angle_measurements[i] - self.target_angles_of_attack[i];

            angle_error = Self::correct_angle_to_be_between_pi_and_negative_pi(angle_error);

            new_local_wing_angles[i] = angle_measurements[i] + angle_error;

            new_local_wing_angles[i] = Self::correct_angle_to_be_between_pi_and_negative_pi(new_local_wing_angles[i]);
        }

        ControllerOutput {
            local_wing_angles: Some(new_local_wing_angles),
            section_models_internal_state: None,
        }
    }

    #[inline(always)]
    fn correct_angle_to_be_between_pi_and_negative_pi(angle: f64) -> f64 {
        let mut corrected_angle = angle;

        while corrected_angle > std::f64::consts::PI {
            corrected_angle -= 2.0 * std::f64::consts::PI;
        }
        while corrected_angle < -std::f64::consts::PI {
            corrected_angle += 2.0 * std::f64::consts::PI;
        }

        corrected_angle
    }
}
