use serde::{Deserialize, Serialize};

use super::moving_average::MovingAverage;

use std::f64::consts::PI;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SailControllerBuilder {
    pub target_angles_of_attack: Vec<f64>,
    pub max_rotational_speed: f64,
    pub angles_in_degrees: bool,
    pub initial_wing_angles: Option<Vec<f64>>,
    pub moving_average_window_size: Option<usize>,
    #[serde(default = "SailControllerBuilder::default_update_factor")]
    pub update_factor: f64,
}

impl SailControllerBuilder {
    pub fn default_update_factor() -> f64 {1.0}

    pub fn build(&self) -> SailController {
        let current_wing_angles = if let Some(initial_angles) = &self.initial_wing_angles {
            initial_angles.clone()
        } else {
            vec![0.0; self.target_angles_of_attack.len()]
        };

        let filters = if let Some(window_size) = self.moving_average_window_size {
            Some(vec![MovingAverage::new(window_size); self.target_angles_of_attack.len()])
        } else {
            None
        };

        SailController {
            target_angles_of_attack: self.target_angles_of_attack.clone(),
            update_factor: self.update_factor,
            max_rotational_speed: self.max_rotational_speed,
            angles_in_degrees: self.angles_in_degrees,
            angle_estimate: vec![0.0; self.target_angles_of_attack.len()],
            filters,
            current_wing_angles,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct SailController {
    pub target_angles_of_attack: Vec<f64>,
    pub update_factor: f64,
    pub max_rotational_speed: f64,
    pub angles_in_degrees: bool,
    pub angle_estimate: Vec<f64>,
    pub filters: Option<Vec<MovingAverage>>,
    pub current_wing_angles: Vec<f64>,
}

impl SailController {
    pub fn compute_new_wing_angles(
        &mut self,
        time_step: f64,
        angle_measurements: &[f64]
    ) -> Vec<f64> {
        self.update_angle_estimate(angle_measurements);

        let angle_errors: Vec<f64> = self.target_angles_of_attack.iter().zip(self.angle_estimate.iter())
            .map(|(target, current)| target - current)
            .collect();

        let raw_angle_changes: Vec<f64> = angle_errors.iter().map(|error| error * self.update_factor).collect();

        let max_angle_change = self.max_rotational_speed * time_step;

        let change_to_apply: Vec<f64> = raw_angle_changes.iter().map(|change| {
            if change.abs() > max_angle_change {
                max_angle_change.copysign(*change)
            } else {
                *change
            }
        }).collect();

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

    pub fn update_angle_estimate(&mut self, angle_measurements: &[f64]) {
        if let Some(filters) = &mut self.filters {
            for i in 0..self.target_angles_of_attack.len() {
                filters[i].add(angle_measurements[i]);
                self.angle_estimate[i] = filters[i].get_average();
            }
        } else {
            self.angle_estimate = angle_measurements.to_vec();
        }
    }
}
