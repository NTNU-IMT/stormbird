use serde::{Deserialize, Serialize};

use stormath::smoothing::moving_average::MovingAverage;

use crate::error::Error;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Builder for the SailController struct
pub struct AngleOfAttackControllerBuilder {
    pub nr_of_wings: usize,
    pub max_rotational_speed: f64,
    pub angles_in_degrees: bool,
    pub moving_average_window_size: Option<usize>,
    #[serde(default = "AngleOfAttackControllerBuilder::default_update_factor")]
    pub update_factor: f64,
    #[serde(default)]
    pub change_threshold: f64,
    #[serde(default)]
    pub max_rotational_speed_to_zero: Option<f64>,
}

impl AngleOfAttackControllerBuilder {
    pub fn default_update_factor() -> f64 {1.0}

    pub fn new_from_file(file_path: &str) -> Result<Self, Error> {
        let file_contents = std::fs::read_to_string(file_path)?;

        let builder = serde_json::from_str(&file_contents)?;

        Ok(builder)
    }

    pub fn build(&self) -> AngleOfAttackController {
        let current_wing_angles = vec![0.0; self.nr_of_wings];
        let angle_estimates = vec![0.0; self.nr_of_wings];

        let filters = if let Some(window_size) = self.moving_average_window_size {
            Some(vec![MovingAverage::new(window_size); self.nr_of_wings])
        } else {
            None
        };

        AngleOfAttackController {
            update_factor: self.update_factor,
            change_threshold: self.change_threshold,
            max_rotational_speed: self.max_rotational_speed,
            max_rotational_speed_to_zero: self.max_rotational_speed_to_zero,
            angles_in_degrees: self.angles_in_degrees,
            angle_estimates,
            filters,
            current_wing_angles,
            time_step_index: 0,
        }
    }
}

#[derive(Debug, Default, Clone)]
/// A sail controller that attempts to set the wing angles to achieve a target effective angle of 
/// attack. 
pub struct AngleOfAttackController {
    /// A factor controlling how quickly the angles of attack are updated
    pub update_factor: f64,
    /// Minimum error in angle of attack that will cause the wing angles to change
    pub change_threshold: f64,
    /// The maximum speed at which the wing angles can change
    pub max_rotational_speed: f64,
    /// The maximum speed to move to zero angle of attack
    pub max_rotational_speed_to_zero: Option<f64>,
    /// Whether the angles of attack are in degrees
    pub angles_in_degrees: bool,
    /// The current estimate of the angles of attack
    pub angle_estimates: Vec<f64>,
    /// Filters to apply to the angle measurements
    pub filters: Option<Vec<MovingAverage>>,
    /// The current wing angles
    pub current_wing_angles: Vec<f64>,
    /// Index to keep track of the number of time steps
    pub time_step_index: usize,
}

impl AngleOfAttackController {
    pub fn new_from_file(file_path: &str) -> Result<Self, Error> {
        let builder = AngleOfAttackControllerBuilder::new_from_file(file_path)?;

        Ok(builder.build())
    }

    pub fn nr_of_wings(&self) -> usize {
        self.angle_estimates.len()
    }

    pub fn compute_new_wing_angles(
        &mut self,
        time_step: f64,
        target_angles_of_attack: &[f64],
        angle_measurements: &[f64]
    ) -> Vec<f64> {
        self.update_angle_estimate(angle_measurements);

        let nr_of_wings = self.nr_of_wings();

        let max_rotational_speed_to_zero = self.max_rotational_speed_to_zero.unwrap_or(self.max_rotational_speed);

        

        for i in 0..nr_of_wings {
            let max_rotational_speed: f64 = if target_angles_of_attack[i] == 0.0 {
                max_rotational_speed_to_zero
            } else {
                self.max_rotational_speed
            };

            let max_angle_change = max_rotational_speed * time_step;

            let angle_error = self.angle_estimates[i] - target_angles_of_attack[i];

            if angle_error.abs() < self.change_threshold {
                continue;
            }

            let raw_angle_change = angle_error * self.update_factor;

            let change_to_apply = if raw_angle_change.abs() > max_angle_change {
                max_angle_change.copysign(raw_angle_change)
            } else {
                raw_angle_change
            };

            self.current_wing_angles[i] = self.current_wing_angles[i] + change_to_apply;
        }

        self.time_step_index += 1;

        self.current_wing_angles.clone()
    }

    pub fn update_angle_estimate(&mut self, angle_measurements: &[f64]) {
        let nr_wings = self.nr_of_wings();

        if let Some(filters) = &mut self.filters {
            for i in 0..nr_wings {
                filters[i].add(angle_measurements[i]);

                self.angle_estimates[i] = filters[i].get_average();
            }
        } else {
            self.angle_estimates = angle_measurements.to_vec();
        }
    }
}
