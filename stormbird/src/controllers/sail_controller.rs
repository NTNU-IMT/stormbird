use serde::{Deserialize, Serialize};

use std::f64::consts::PI;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct SailControllerBuilder {
    pub target_angles_of_attack: Vec<f64>,
    pub max_rotational_speed: f64,
    pub angles_in_degrees: bool,
    pub initial_wing_angles: Option<Vec<f64>>,
}

impl SailControllerBuilder {
    pub fn build(&self) -> SailController {
        let current_wing_angles = if let Some(initial_angles) = &self.initial_wing_angles {
            initial_angles.clone()
        } else {
            vec![0.0; self.target_angles_of_attack.len()]
        };

        SailController {
            target_angles_of_attack: self.target_angles_of_attack.clone(),
            max_rotational_speed: self.max_rotational_speed,
            angles_in_degrees: self.angles_in_degrees,
            current_wing_angles,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct SailController {
    pub target_angles_of_attack: Vec<f64>,
    pub max_rotational_speed: f64,
    pub angles_in_degrees: bool,

    current_wing_angles: Vec<f64>,
}

impl SailController {
    pub fn compute_new_wing_angles(
        &mut self,
        time_step: f64,
        angle_measurments: &[f64]
    ) -> Vec<f64> {
        let angle_errors: Vec<f64> = self.target_angles_of_attack.iter().zip(angle_measurments)
            .map(|(target, current)| target - current)
            .collect();

        let max_angle_change = self.max_rotational_speed * time_step;

        let change_to_apply: Vec<f64> = angle_errors.iter()
            .map(
                |error| {
                    if error.abs() > max_angle_change {
                        max_angle_change.copysign(*error)
                    } else {
                        *error
                    }
                }
            ).collect();

        let angle_limit = if self.angles_in_degrees {
            180.0
        } else {
            PI
        };

        self.current_wing_angles = self.current_wing_angles.iter().zip(change_to_apply.iter()).map(
            |(current, change)| {
                let mut new_angle = *current - *change;

                if new_angle > angle_limit {
                    new_angle -= 2.0 * angle_limit;
                } else if new_angle < -angle_limit {
                    new_angle += 2.0 * angle_limit;
                }

                new_angle
            }
        ).collect();

        self.current_wing_angles.clone()
    }
}
