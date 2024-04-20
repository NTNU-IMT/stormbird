// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use crate::math_utils::interpolation::linear_interpolation;

use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaryingFoil {
    pub variable_value: f64,
    pub variable_data: Vec<f64>,
    pub foils_data: Vec<Foil>,
}

impl VaryingFoil {
    pub fn get_foil(&self) -> Foil {
        let cl_zero_angle_data: Vec<f64> = self.foils_data.iter().map(|x| x.cl_zero_angle).collect();
        let cl_initial_slope_data: Vec<f64> = self.foils_data.iter().map(|x| x.cl_initial_slope).collect();
        let cd_zero_angle_data: Vec<f64> = self.foils_data.iter().map(|x| x.cd_zero_angle).collect();
        let cl_high_order_factor_data: Vec<f64> = self.foils_data.iter().map(|x| x.cl_high_order_factor).collect();
        let cl_high_order_power_data: Vec<f64> = self.foils_data.iter().map(|x| x.cl_high_order_power).collect();
        let cd_second_order_factor_data: Vec<f64> = self.foils_data.iter().map(|x| x.cd_second_order_factor).collect();
        let cl_max_after_stall_data: Vec<f64> = self.foils_data.iter().map(|x| x.cl_max_after_stall).collect();
        let cd_max_after_stall_data: Vec<f64> = self.foils_data.iter().map(|x| x.cd_max_after_stall).collect();
        let cd_power_after_stall_data: Vec<f64> = self.foils_data.iter().map(|x| x.cd_power_after_stall).collect();
        let mean_stall_angle_data: Vec<f64> = self.foils_data.iter().map(|x| x.mean_stall_angle).collect();
        let stall_range_data: Vec<f64> = self.foils_data.iter().map(|x| x.stall_range).collect();

        Foil {
            cl_zero_angle: linear_interpolation(self.variable_value, &self.variable_data, &cl_zero_angle_data),
            cl_initial_slope: linear_interpolation(self.variable_value, &self.variable_data, &cl_initial_slope_data),
            cd_zero_angle: linear_interpolation(self.variable_value, &self.variable_data, &cd_zero_angle_data),
            cl_high_order_factor: linear_interpolation(self.variable_value, &self.variable_data, &cl_high_order_factor_data),
            cl_high_order_power: linear_interpolation(self.variable_value, &self.variable_data, &cl_high_order_power_data),
            cd_second_order_factor: linear_interpolation(self.variable_value, &self.variable_data, &cd_second_order_factor_data),
            cl_max_after_stall: linear_interpolation(self.variable_value, &self.variable_data, &cl_max_after_stall_data),
            cd_max_after_stall: linear_interpolation(self.variable_value, &self.variable_data, &cd_max_after_stall_data),
            cd_power_after_stall: linear_interpolation(self.variable_value, &self.variable_data, &cd_power_after_stall_data),
            mean_stall_angle: linear_interpolation(self.variable_value, &self.variable_data, &mean_stall_angle_data),
            stall_range: linear_interpolation(self.variable_value, &self.variable_data, &stall_range_data),
        }
    }

    pub fn lift_coefficient(&self, angle_of_attack: f64) -> f64 {
        self.get_foil().lift_coefficient(angle_of_attack)
    }

    pub fn drag_coefficient(&self, angle_of_attack: f64) -> f64 {
        self.get_foil().drag_coefficient(angle_of_attack)
    }
}