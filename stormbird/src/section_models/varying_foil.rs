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
    #[serde(default)]
    pub current_internal_state: Float,
    #[serde(default)]
    pub current_foil: Option<Foil>,
}

impl VaryingFoil {
    pub fn new_from_string(input_str: &str) -> Self {
        let data: VaryingFoil = serde_json::from_str(input_str).unwrap();
        
        data
    }

    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    pub fn get_foil(&self) -> Foil {
        let cl_zero_angle_data: Vec<Float> = self.foils_data.iter().map(|x| x.cl_zero_angle).collect();
        let cl_initial_slope_data: Vec<Float> = self.foils_data.iter().map(|x| x.cl_initial_slope).collect();
        let cl_high_order_factor_data: Vec<Float> = self.foils_data.iter().map(|x| x.cl_high_order_factor).collect();
        let cl_high_order_power_data: Vec<Float> = self.foils_data.iter().map(|x| x.cl_high_order_power).collect();
        let cl_max_after_stall_data: Vec<Float> = self.foils_data.iter().map(|x| x.cl_max_after_stall).collect();

        let cd_min_data: Vec<Float> = self.foils_data.iter().map(|x| x.cd_min).collect();
        let angle_cd_min_data: Vec<Float> = self.foils_data.iter().map(|x| x.angle_cd_min).collect();
        let cd_second_order_factor_data: Vec<Float> = self.foils_data.iter().map(|x| x.cd_second_order_factor).collect();
        let cd_max_after_stall_data: Vec<Float> = self.foils_data.iter().map(|x| x.cd_max_after_stall).collect();
        let cd_power_after_stall_data: Vec<Float> = self.foils_data.iter().map(|x| x.cd_power_after_stall).collect();
        let cdi_correction_factor_data: Vec<Float> = self.foils_data.iter().map(|x| x.cdi_correction_factor).collect();

        let mean_positive_stall_angle_data: Vec<Float> = self.foils_data.iter().map(|x| x.mean_positive_stall_angle).collect();
        let mean_negative_stall_angle_data: Vec<Float> = self.foils_data.iter().map(|x| x.mean_negative_stall_angle).collect();
        let stall_range_data: Vec<Float> = self.foils_data.iter().map(|x| x.stall_range).collect();

        let added_mass_factor_data: Vec<Float> = self.foils_data.iter().map(|x| x.added_mass_factor).collect();

        let x = self.current_internal_state;
        let x_data = &self.internal_state_data;

        let stall_model = self.foils_data[0].stall_model.clone();

        Foil {
            cl_zero_angle:          linear_interpolation(x, x_data, &cl_zero_angle_data),
            cl_initial_slope:       linear_interpolation(x, x_data, &cl_initial_slope_data),
            cl_high_order_factor:   linear_interpolation(x, x_data, &cl_high_order_factor_data),
            cl_high_order_power:    linear_interpolation(x, x_data, &cl_high_order_power_data),
            cl_max_after_stall:     linear_interpolation(x, x_data, &cl_max_after_stall_data),
            cd_min:                 linear_interpolation(x, x_data, &cd_min_data),
            angle_cd_min:          linear_interpolation(x, x_data, &angle_cd_min_data),
            cd_second_order_factor: linear_interpolation(x, x_data, &cd_second_order_factor_data),
            cd_max_after_stall:     linear_interpolation(x, x_data, &cd_max_after_stall_data),
            cd_power_after_stall:   linear_interpolation(x, x_data, &cd_power_after_stall_data),
            cdi_correction_factor:  linear_interpolation(x, x_data, &cdi_correction_factor_data),
            mean_positive_stall_angle: linear_interpolation(x, x_data, &mean_positive_stall_angle_data),
            mean_negative_stall_angle: linear_interpolation(x, x_data, &mean_negative_stall_angle_data),
            stall_range:            linear_interpolation(x, x_data, &stall_range_data),
            added_mass_factor:      linear_interpolation(x, x_data, &added_mass_factor_data),
            stall_model:            stall_model,
        }
    }

    pub fn set_internal_state(&mut self, internal_state: Float) {
        self.current_internal_state = internal_state;
        self.current_foil = Some(self.get_foil());
    }

    pub fn lift_coefficient(&self, angle_of_attack: Float) -> Float {
        self.get_foil().lift_coefficient(angle_of_attack)
    }

    pub fn drag_coefficient(&self, angle_of_attack: Float) -> Float {
        self.get_foil().drag_coefficient(angle_of_attack)
    }

    pub fn added_mass_coefficient(&self, heave_acceleration: Float) -> Float {
        self.get_foil().added_mass_coefficient(heave_acceleration)
    }

    pub fn amount_of_stall(&self, angle_of_attack: Float) -> Float {
        self.get_foil().amount_of_stall(angle_of_attack)
    }
}