// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use super::*;
use super::common_functions;

use std::f64::consts::PI;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Parametric model of a single element foil profile
pub struct Foil {
    #[serde(default)]
    pub cl_zero_angle: f64,
    #[serde(default)]
    pub cd_zero_angle: f64,

    #[serde(default)]
    pub cl_high_order_factor: f64,
    #[serde(default)]
    pub cl_high_order_power: f64,

    #[serde(default)]
    pub cd_second_order_factor: f64,

    #[serde(default="Foil::default_one")]
    pub cl_max_after_stall: f64,
    #[serde(default="Foil::default_one")]
    pub cd_max_after_stall: f64,
    #[serde(default="Foil::default_cd_power_after_stall")]
    pub cd_power_after_stall: f64,

    #[serde(default="Foil::default_mean_stall_angle")]
    pub mean_stall_angle: f64,
    #[serde(default="Foil::default_stall_range")]
    pub stall_range: f64,
}



fn get_stall_angle(angle_of_attack: f64) -> f64 {
    let mut effective = angle_of_attack.abs();

        while effective > PI {
            effective -= PI;
        }

        effective *= angle_of_attack.signum();
    
    effective
}

impl Foil {
    pub fn default_one()                  -> f64 {1.0}
    pub fn default_mean_stall_angle()     -> f64 {20.0_f64.to_radians()}
    pub fn default_stall_range()          -> f64 {6.0_f64.to_radians()}
    pub fn default_cd_power_after_stall() -> f64 {1.6}
    
    pub fn lift_coefficient(&self, angle_of_attack: f64) -> f64 {
        let stall_angle = get_stall_angle(angle_of_attack);

        let cl_a1 = 2.0 * PI;

        let angle_high_power = if self.cl_high_order_power > 0.0 {
            angle_of_attack.powf(self.cl_high_order_power)
        } else {
            0.0
        };

        let cl_pre_stall  = 
            self.cl_zero_angle + 
            cl_a1 * angle_of_attack +
            self.cl_high_order_factor * angle_high_power;


        let cl_post_stall = self.cl_max_after_stall * (2.0 * stall_angle).sin();

        self.combine_pre_and_post_stall(angle_of_attack.abs(), cl_pre_stall, cl_post_stall)
    }

    pub fn drag_coefficient(&self, angle_of_attack: f64) -> f64 {
        let stall_angle = get_stall_angle(angle_of_attack);

        let cd_pre_stall  = self.cd_zero_angle + self.cd_second_order_factor * angle_of_attack.powi(2);
        let cd_post_stall = self.cd_max_after_stall * stall_angle.sin().abs().powf(self.cd_power_after_stall);

        self.combine_pre_and_post_stall(angle_of_attack.abs(), cd_pre_stall, cd_post_stall)
    }

    fn combine_pre_and_post_stall(&self, angle_of_attack: f64, pre_stall: f64, post_stall: f64) -> f64 {
        let amount_of_stall = common_functions::sigmoid_function(
            angle_of_attack, 
            self.mean_stall_angle, 
            self.stall_range
        );

        pre_stall * (1.0 - amount_of_stall) + amount_of_stall * post_stall
    }
}

impl Default for Foil {
    fn default() -> Self {
        Self {
            cl_zero_angle:          0.0,
            cd_zero_angle:          0.0,
            cl_high_order_factor:   0.0,
            cl_high_order_power:    0.0,
            cd_second_order_factor: 0.0,
            cl_max_after_stall:     1.0,
            cd_max_after_stall:     1.0,
            cd_power_after_stall:   Self::default_cd_power_after_stall(),
            mean_stall_angle:       Self::default_mean_stall_angle(),
            stall_range:            Self::default_stall_range()
        }
    }
}