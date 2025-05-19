use serde::{Deserialize, Serialize};

use stormath::{
    smoothing::moving_average::MovingAverage,
    statistics
};

use crate::error::Error;
use crate::io_utils::csv_data;
use crate::common_utils::results::simulation::SimulationResult;
use super::{
    LineForceModelState,
    ControllerOutput
};

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
pub enum AngleMeasurmentType {
    Max,
    #[default]
    Mean
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Builder for the SailController struct
pub struct EffectiveAngleOfAttackControllerBuilder {
    pub target_angles_of_attack: Vec<f64>,
    #[serde(default = "EffectiveAngleOfAttackControllerBuilder::default_time_steps_between_updates")]
    pub time_steps_between_updates: usize,
    #[serde(default)]
    pub max_rotational_speed: Option<f64>,
    #[serde(default)]
    pub moving_average_window_size: Option<usize>,
    #[serde(default = "EffectiveAngleOfAttackControllerBuilder::default_update_factor")]
    pub update_factor: f64,
    #[serde(default = "EffectiveAngleOfAttackControllerBuilder::default_change_threshold")]
    pub change_threshold: f64,
    #[serde(default)]
    pub angle_measurment_start_index: usize,
    #[serde(default)]
    pub angle_measurment_end_offset: usize,
    #[serde(default)]
    pub angle_measurment_type: AngleMeasurmentType,
}

impl EffectiveAngleOfAttackControllerBuilder {
    pub fn default_time_steps_between_updates() -> usize {1}
    pub fn default_update_factor() -> f64 {1.0}
    pub fn default_change_threshold() -> f64 {0.0001_f64.to_radians()}

    pub fn new_from_file(file_path: &str) -> Result<Self, Error> {
        let file_contents = std::fs::read_to_string(file_path)?;

        let builder = serde_json::from_str(&file_contents)?;

        Ok(builder)
    }

    pub fn build(&self) -> EffectiveAngleOfAttackController {
        let nr_of_wings = self.target_angles_of_attack.len();

        let angle_estimates = vec![0.0; nr_of_wings];

        let filters = if let Some(window_size) = self.moving_average_window_size {
            Some(vec![MovingAverage::new(window_size); nr_of_wings])
        } else {
            None
        };

        EffectiveAngleOfAttackController {
            target_angles_of_attack: self.target_angles_of_attack.clone(),
            time_steps_between_updates: self.time_steps_between_updates,
            update_factor: self.update_factor,
            change_threshold: self.change_threshold,
            max_rotational_speed: self.max_rotational_speed,
            angle_estimates,
            filters,
            time_step_index: 0,
            angle_measurment_start_index: self.angle_measurment_start_index,
            angle_measurment_end_offset: self.angle_measurment_end_offset,
            angle_measurment_type: self.angle_measurment_type,
        }
    }
}

#[derive(Debug, Default, Clone)]
/// A sail controller that attempts to set the wing angles to achieve a target effective angle of 
/// attack. 
pub struct EffectiveAngleOfAttackController {
    /// The set point for the effective angle of attack
    pub target_angles_of_attack: Vec<f64>,
    /// How often to update the angles of attack
    pub time_steps_between_updates: usize,
    /// A factor controlling how quickly the angles of attack are updated
    pub update_factor: f64,
    /// Minimum error in angle of attack that will cause the wing angles to change
    pub change_threshold: f64,
    /// The maximum speed at which the wing angles can change
    pub max_rotational_speed: Option<f64>,
    /// The current estimate of the angles of attack
    pub angle_estimates: Vec<f64>,
    /// Filters to apply to the angle measurements
    pub filters: Option<Vec<MovingAverage>>,
    /// Index to keep track of the number of time steps
    pub time_step_index: usize,
    /// At which index to start the angle measurements
    pub angle_measurment_start_index: usize,
    /// An offset used to compute the final index of the angle measurements
    pub angle_measurment_end_offset: usize,
    /// The type of angle measurement to use
    pub angle_measurment_type: AngleMeasurmentType,
}

impl EffectiveAngleOfAttackController {
    pub fn new_from_file(file_path: &str) -> Result<Self, Error> {
        let builder = EffectiveAngleOfAttackControllerBuilder::new_from_file(file_path)?;

        Ok(builder.build())
    }

    fn nr_of_wings(&self) -> usize {
        self.angle_estimates.len()
    }

    fn measure_angles_of_attack(&self, simulation_result: &SimulationResult) -> Vec<f64> {
        let mut angles_of_attack = vec![0.0; self.nr_of_wings()];

        for i in 0..self.nr_of_wings() {
            let wing_angles_of_attack = simulation_result.angles_of_attack_for_wing(i);
            let nr_strips = wing_angles_of_attack.len();

            let start_index = self.angle_measurment_start_index;
            let end_index = (nr_strips - self.angle_measurment_end_offset - 1).max(start_index+1).min(nr_strips-1);

            angles_of_attack[i] = match self.angle_measurment_type {
                AngleMeasurmentType::Mean => statistics::mean(
                    &wing_angles_of_attack[start_index..end_index]
                ),
                AngleMeasurmentType::Max => {
                    if self.target_angles_of_attack[i] > 0.0 {
                        statistics::max(
                            &wing_angles_of_attack[start_index..end_index]
                        )
                    } else {
                        statistics::min(
                            &wing_angles_of_attack[start_index..end_index]
                        )
                    }
                },
            };
        }

        angles_of_attack
    }

    pub fn update(
        &mut self, 
        time_step: f64, 
        model_state: &LineForceModelState, 
        simulation_result: &SimulationResult
    ) -> Option<ControllerOutput> {
        self.time_step_index += 1;

        let angle_measurements = self.measure_angles_of_attack(simulation_result);

        self.update_angle_estimate(&angle_measurements);

        self.write_angle_data_to_file(&angle_measurements);

        let time_to_update =  self.time_step_index % self.time_steps_between_updates == 0;
        let first_time_step = self.time_step_index == 1;

        if first_time_step || time_to_update {
            let new_local_wing_angles = self.compute_new_local_wing_angles(
                time_step,
                model_state,
                first_time_step
            );

            let change_necessary = self.check_for_change(
                &model_state.local_wing_angles,
                &new_local_wing_angles
            );

            if change_necessary {
                Some(
                    ControllerOutput {
                        local_wing_angles: Some(new_local_wing_angles),
                        section_models_internal_state: None,
                    }
                )
            } else {
                None
            }
        } else {
            None
        }
    }

    fn check_for_change(
        &self, 
        old_local_wing_angles: &[f64], 
        new_local_wing_angels: &[f64]
    ) -> bool {
        let mut change_necessary = false;
        
        for i in 0..self.nr_of_wings() {
            let angle_error = new_local_wing_angels[i] - old_local_wing_angles[i];

            if angle_error.abs() > self.change_threshold {
                change_necessary =  true;
            }
        }

        change_necessary
    }

    fn compute_new_local_wing_angles(
        &self, 
        time_step: f64, 
        model_state: &LineForceModelState,
        first_time_step: bool
    ) -> Vec<f64> {
        let mut new_local_wing_angles = vec![0.0; self.nr_of_wings()];

        let nr_of_wings = self.nr_of_wings();

        let max_angle_change = if let Some(max_rotational_speed) = self.max_rotational_speed {
            max_rotational_speed * time_step
        } else {
            std::f64::MAX
        };

        for i in 0..nr_of_wings {
            let mut angle_error = self.angle_estimates[i] - self.target_angles_of_attack[i];

            angle_error = Self::correct_angle_to_be_between_pi_and_negative_pi(angle_error);

            let raw_angle_change = if first_time_step {
                angle_error
            } else {
                angle_error * self.update_factor
            };

            let change_to_apply = if raw_angle_change.abs() > max_angle_change {
                max_angle_change.copysign(raw_angle_change)
            } else {
                raw_angle_change
            };

            new_local_wing_angles[i] = model_state.local_wing_angles[i] + change_to_apply;

            new_local_wing_angles[i] = Self::correct_angle_to_be_between_pi_and_negative_pi(new_local_wing_angles[i]);
        }

        new_local_wing_angles
    }

    fn update_angle_estimate(&mut self, angle_measurements: &[f64]) {
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

    fn angle_data_as_csv_string(&self, angle_measurements: &[f64]) -> (String, String) {
        let mut header = String::new();
        let mut data = String::new();

        for i in 0..self.nr_of_wings() {
            if i > 0 {
                header.push(',');
                data.push(',');
            }

            header.push_str(&format!("angle_estimate_{},", i));
            data.push_str(&format!("{},", self.angle_estimates[i]));

            header.push_str(&format!("target_angle_{},", i));
            data.push_str(&format!("{},", self.target_angles_of_attack[i]));

            header.push_str(&format!("angle_measurement_{}", i));
            data.push_str(&format!("{}", angle_measurements[i]));
        }

        (header, data)
    }

    fn write_angle_data_to_file(
        &self, 
        angle_measurements: &[f64]
    ) {
        let (header, data) = self.angle_data_as_csv_string(angle_measurements);

        let _ = csv_data::create_or_append_header_and_data_strings_file(
            "angle_data.csv",
            &header,
            &data
        );
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
