
use serde::{Deserialize, Serialize};

use stormath::interpolation::linear_interpolation;

use super::prelude::*;

use stormath::type_aliases::Float;
use stormath::consts::{PI, TAU};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpinRatioConversion {
    diameter: Float,
    max_rps: Float,
}

impl SpinRatioConversion {
    pub fn get_rps_from_spin_ratio(&self, spin_ratio: Float, velocity: Float) -> Float {
        let circumference = PI * self.diameter;

        let rps_raw = spin_ratio * velocity / circumference;

        if rps_raw.abs() > self.max_rps {
            self.max_rps * rps_raw.signum()
        } else {
            rps_raw
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub enum InternalStateType {
    #[default]
    Generic,
    SpinRatio(SpinRatioConversion),
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Controller that interpolate on arrays of weather-dependent set points
pub struct ControllerLogic {
    pub apparent_wind_directions_data: Vec<Float>,
    #[serde(default)]
    pub angle_of_attack_set_points_data: Option<Vec<Float>>,
    #[serde(default)]
    pub section_model_internal_state_set_points_data: Option<Vec<Float>>,
    #[serde(default)]
    pub internal_state_type: InternalStateType,
    #[serde(default)]
    pub use_effective_angle_of_attack: bool,
}

impl ControllerLogic {
    pub fn get_new_output(&self, input: &ControllerInput) -> ControllerOutput {
        let local_wing_angles = if self.use_effective_angle_of_attack {
            self.get_local_wing_angles_effective(input)
        } else {
            self.get_local_wing_angles_geometric(input)
        };

        let section_models_internal_state = self.get_section_models_internal_state(input);

        ControllerOutput {
            local_wing_angles,
            section_models_internal_state,
        }
    }

    pub fn get_local_wing_angles_geometric(&self, input: &ControllerInput) -> Option<Vec<Float>> {
        let nr_of_wings = input.apparent_wind_directions.len();
        
        if self.angle_of_attack_set_points_data.is_some() {
            let mut out = Vec::with_capacity(nr_of_wings);

            for i in 0..nr_of_wings {
                let set_point = input.loading * self.get_angle_of_attack_set_points(
                    input.apparent_wind_directions[i]
                ).unwrap();

                let wing_angle = input.apparent_wind_directions[i] + set_point;

                out.push(wing_angle);
            }

            Some(out)
        } else {
            None
        }
    }

    pub fn get_local_wing_angles_effective(&self, input: &ControllerInput) -> Option<Vec<Float>> {
        let nr_of_wings = input.apparent_wind_directions.len();

        let angle_measurements = &input.angles_of_attack;

        if self.angle_of_attack_set_points_data.is_some() {
            let mut out = Vec::with_capacity(nr_of_wings);

            for i in 0..nr_of_wings {
                let set_point = input.loading * self.get_angle_of_attack_set_points(
                    input.apparent_wind_directions[i]
                ).unwrap();

                let mut angle_error = angle_measurements[i] - set_point;

                angle_error = Self::correct_angle_to_be_between_pi_and_negative_pi(angle_error);

                out[i] = angle_measurements[i] + angle_error;

                out[i] = Self::correct_angle_to_be_between_pi_and_negative_pi(out[i]);
            }

            Some(out)
        } else {
            None
        }
    }

    pub fn get_section_models_internal_state(&self, input: &ControllerInput) -> Option<Vec<Float>> {
        let nr_of_wings = input.apparent_wind_directions.len();

        if self.section_model_internal_state_set_points_data.is_some() {
            let mut out = Vec::with_capacity(nr_of_wings);
            
            for i in 0..nr_of_wings {
                let internal_state_raw = input.loading * self.get_internal_state_set_points(
                    input.apparent_wind_directions[i]
                ).unwrap();

                let internal_state = match self.internal_state_type {
                    InternalStateType::Generic => {internal_state_raw}
                    InternalStateType::SpinRatio(ref conversion) => {
                        let velocity = input.velocity[i];

                        conversion.get_rps_from_spin_ratio(internal_state_raw, velocity)
                    }
                };
                
                out.push(internal_state);
            }

            Some(out)

        } else {
            None
        }
    }

    pub fn get_angle_of_attack_set_points(&self, apparent_wind_direction: Float) -> Option<Float> {
        if let Some(angle_data) = &self.angle_of_attack_set_points_data {
            return Some(
                linear_interpolation(
                    apparent_wind_direction,
                    &self.apparent_wind_directions_data,
                    angle_data,
                )
            );
        } else {
            return None;
        }
    }

    pub fn get_internal_state_set_points(&self, apparent_wind_direction: Float) -> Option<Float> {
        if let Some(internal_states_data) = &self.section_model_internal_state_set_points_data {
            return Some(
                linear_interpolation(
                    apparent_wind_direction,
                    &self.apparent_wind_directions_data,
                    internal_states_data,
                )
            )
        } else {
            return None
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