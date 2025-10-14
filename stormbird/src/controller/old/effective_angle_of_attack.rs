// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)


use serde::{Deserialize, Serialize};

use super::prelude::*;

use stormath::type_aliases::Float;
use stormath::consts::{PI, TAU};


#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// A sail controller that attempts to set the wing angles to achieve a target effective angle of 
/// attack. 
pub struct EffectiveAngleOfAttackController {
    /// The set point for the effective angle of attack
    pub target_angles_of_attack: Vec<Float>,
}

impl EffectiveAngleOfAttackController {
    pub fn get_new_output(
        &self,
        loading: Float,
        angle_measurements: &[Float]
    ) -> ControllerOutput {
        let nr_of_wings = angle_measurements.len();

        let mut new_local_wing_angles = vec![0.0; nr_of_wings];

        for i in 0..nr_of_wings {
            let mut angle_error = angle_measurements[i] - self.target_angles_of_attack[i] * loading;

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
    fn correct_angle_to_be_between_pi_and_negative_pi(angle: Float) -> Float {
        let mut corrected_angle = angle;

        while corrected_angle > PI {
            corrected_angle -= TAU;
        }
        while corrected_angle < -PI {
            corrected_angle += TAU;
        }

        corrected_angle
    }
}
