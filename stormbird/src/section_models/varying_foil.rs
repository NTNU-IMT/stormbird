// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use stormath::interpolation::linear_interpolation;

use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
/// A foil profile where the parameters can vary depending on an internal state. 
/// 
/// The two typical use cases are to model foil sections that include a flap angle, or suction 
/// sails, where the foil section properties are dependent on the suction rate.
pub struct VaryingFoil {
    pub internal_state_data: Vec<Float>,
    pub foils_data: Vec<Foil>,
}

impl VaryingFoil {
    pub fn new_from_string(input_str: &str) -> Self {
        let data: VaryingFoil = serde_json::from_str(input_str).unwrap();
        
        data
    }

    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    pub fn get_foil(&self, internal_state: Float) -> Foil {
        let cl_zero_angle_data: Vec<Float> = self.foils_data.iter().map(|x| x.cl_zero_angle).collect();
        let cl_initial_slope_data: Vec<Float> = self.foils_data.iter().map(|x| x.cl_initial_slope).collect();
        let cl_slope_reduction_factor_positive_data: Vec<Float> = self.foils_data.iter().map(|x| x.cl_slope_reduction_factor_positive).collect();
        let cl_slope_reduction_factor_negative_data: Vec<Float> = self.foils_data.iter().map(|x| x.cl_slope_reduction_factor_negative).collect();
        let cl_slope_reduction_onset_angle_positive_data: Vec<Float> = self.foils_data.iter().map(|x| x.cl_slope_reduction_onset_angle_positive).collect();
        let cl_slope_reduction_onset_angle_negative_data: Vec<Float> = self.foils_data.iter().map(|x| x.cl_slope_reduction_onset_angle_negative).collect();
        let cl_slope_reduction_range_data: Vec<Float> = self.foils_data.iter().map(|x| x.cl_slope_reduction_range).collect();
        let cl_max_after_stall_data: Vec<Float> = self.foils_data.iter().map(|x| x.cl_max_after_stall).collect();

        let cd_min_data: Vec<Float> = self.foils_data.iter().map(|x| x.cd_min).collect();
        let cd_second_order_factor_data: Vec<Float> = self.foils_data.iter().map(|x| x.cd_second_order_factor).collect();
        let cd_max_after_stall_data: Vec<Float> = self.foils_data.iter().map(|x| x.cd_max_after_stall).collect();
        let cd_power_after_stall_data: Vec<Float> = self.foils_data.iter().map(|x| x.cd_power_after_stall).collect();
        let cdi_correction_factor_data: Vec<Float> = self.foils_data.iter().map(|x| x.cdi_correction_factor).collect();

        let mean_positive_stall_angle_data: Vec<Float> = self.foils_data.iter().map(|x| x.mean_positive_stall_angle).collect();
        let mean_negative_stall_angle_data: Vec<Float> = self.foils_data.iter().map(|x| x.mean_negative_stall_angle).collect();
        let stall_range_data: Vec<Float> = self.foils_data.iter().map(|x| x.stall_range).collect();
        let cd_stall_angle_offset_data: Vec<Float> = self.foils_data.iter().map(|x| x.cd_stall_angle_offset).collect();
        let cd_bump_during_stall_data: Vec<Float> = self.foils_data.iter().map(|x| x.cd_bump_during_stall).collect();

        let added_mass_factor_data: Vec<Float> = self.foils_data.iter().map(|x| x.added_mass_factor).collect();

        let x = internal_state;
        let x_data = &self.internal_state_data;

        Foil {
            cl_zero_angle:             linear_interpolation(x, x_data, &cl_zero_angle_data),
            cl_initial_slope:          linear_interpolation(x, x_data, &cl_initial_slope_data),
            cl_slope_reduction_factor_positive: linear_interpolation(x, x_data, &cl_slope_reduction_factor_positive_data),
            cl_slope_reduction_factor_negative: linear_interpolation(x, x_data, &cl_slope_reduction_factor_negative_data),
            cl_slope_reduction_onset_angle_positive: linear_interpolation(x, x_data, &cl_slope_reduction_onset_angle_positive_data),
            cl_slope_reduction_onset_angle_negative: linear_interpolation(x, x_data, &cl_slope_reduction_onset_angle_negative_data),
            cl_slope_reduction_range:  linear_interpolation(x, x_data, &cl_slope_reduction_range_data),
            cl_max_after_stall:        linear_interpolation(x, x_data, &cl_max_after_stall_data),
            cd_min:                    linear_interpolation(x, x_data, &cd_min_data),
            cd_second_order_factor:    linear_interpolation(x, x_data, &cd_second_order_factor_data),
            cd_max_after_stall:        linear_interpolation(x, x_data, &cd_max_after_stall_data),
            cd_power_after_stall:      linear_interpolation(x, x_data, &cd_power_after_stall_data),
            cdi_correction_factor:     linear_interpolation(x, x_data, &cdi_correction_factor_data),
            mean_positive_stall_angle: linear_interpolation(x, x_data, &mean_positive_stall_angle_data),
            mean_negative_stall_angle: linear_interpolation(x, x_data, &mean_negative_stall_angle_data),
            stall_range:               linear_interpolation(x, x_data, &stall_range_data),
            cd_stall_angle_offset:     linear_interpolation(x, x_data, &cd_stall_angle_offset_data),
            cd_bump_during_stall:      linear_interpolation(x, x_data, &cd_bump_during_stall_data),
            added_mass_factor:         linear_interpolation(x, x_data, &added_mass_factor_data)
        }
    }

    pub fn lift_coefficient(&self, angle_of_attack: Float, internal_state: Float) -> Float {
        self.get_foil(internal_state).lift_coefficient(angle_of_attack)
    }

    pub fn lift_coefficient_linear(&self, angle_of_attack: Float, internal_state: Float) -> Float {
        self.get_foil(internal_state).lift_coefficient_linear(angle_of_attack)
    }

    pub fn lift_coefficient_pre_stall_raw(&self, angle_of_attack: Float, internal_state: Float) -> Float {
        self.get_foil(internal_state).lift_coefficient_pre_stall_raw(angle_of_attack)
    }

    pub fn lift_coefficient_pre_stall_with_stall_drop_off(&self, angle_of_attack: Float, internal_state: Float) -> Float {
        self.get_foil(internal_state).lift_coefficient_pre_stall_with_stall_drop_off(angle_of_attack)
    }

    pub fn lift_coefficient_post_stall_raw(&self, angle_of_attack: Float, internal_state: Float) -> Float {
        self.get_foil(internal_state).lift_coefficient_post_stall_raw(angle_of_attack)
    }

    pub fn lift_coefficient_post_stall_with_stall_weight(&self, angle_of_attack: Float, internal_state: Float) -> Float {
        self.get_foil(internal_state).lift_coefficient_post_stall_with_stall_weight(angle_of_attack)
    }

    pub fn lift_coefficient_derivative(&self, internal_state: Float) -> Float {
        self.get_foil(internal_state).cl_initial_slope
    }

    pub fn drag_coefficient(&self, angle_of_attack: Float, internal_state: Float) -> Float {
        self.get_foil(internal_state).drag_coefficient(angle_of_attack)
    }

    pub fn added_mass_coefficient(&self, heave_acceleration: Float, internal_state: Float) -> Float {
        self.get_foil(internal_state).added_mass_coefficient(heave_acceleration)
    }

    pub fn amount_of_stall(&self, angle_of_attack: Float, internal_state: Float) -> Float {
        self.get_foil(internal_state).amount_of_stall(angle_of_attack)
    }
}
