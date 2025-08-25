// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use stormath::{
    consts::PI, 
    spatial_vector::SpatialVector, 
    type_aliases::Float
};

use crate::wind::wind_condition::WindCondition;

use crate::error::Error;

use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Parametric model for wind loads on a ship superstructure.
///
/// Based on the "Blendermann model".
/// Source: W. Blendermann, 1994, Parameter identification of wind loads on ships,
/// Journal of Wind Engineering and Industrial Aerodynamics, 51, 339-351.
/// Link: https://www.sciencedirect.com/science/article/pii/0167610594900671
pub struct BlendermannSuperstructureForces {
    pub frontal_area: Float,
    pub side_area: Float,
    pub ship_length: Float,
    #[serde(default="BlendermannSuperstructureForces::default_resistance_coefficient")]
    pub resistance_coefficient: Float,
    #[serde(default="BlendermannSuperstructureForces::default_side_force_coefficient")]
    pub side_force_coefficient: Float,
    #[serde(default)]
    pub non_dim_side_area_centroid: Float,
    #[serde(default)]
    pub non_dim_heel_arm: Float,
    #[serde(default="BlendermannSuperstructureForces::default_coupling_factor")]
    pub coupling_factor: Float,
    #[serde(default="BlendermannSuperstructureForces::default_non_dim_yaw_arm_correction")]
    pub non_dim_yaw_arm_correction: Float,
    #[serde(default="BlendermannSuperstructureForces::default_air_density")]
    pub density: Float,
    #[serde(default="default_one")]
    pub velocity_correction_factor: Float,
}

fn default_one() -> Float {1.0}

impl BlendermannSuperstructureForces {
    fn default_resistance_coefficient() -> Float {0.55}
    fn default_side_force_coefficient() -> Float {0.85}
    fn default_coupling_factor() -> Float {0.55}
    fn default_non_dim_yaw_arm_correction() -> Float {0.0}
    fn default_air_density() -> Float {1.225}

    pub fn from_json_string(json_string: &str) -> Result<Self, Error> {
        let serde_res = serde_json::from_str(json_string)?;
        
        Ok(serde_res)
    }

    pub fn from_json_file(file_path: &str) -> Result<Self, Error> {
        let json_string = std::fs::read_to_string(file_path)?;
        
        Self::from_json_string(&json_string)
    }

    pub fn body_fixed_force(&self, apparent_wind: WindCondition) -> SpatialVector {
        SpatialVector::new(
            -self.resistance(&apparent_wind),
            self.side_force(&apparent_wind),
            0.0
        )
    }

    pub fn body_fixed_moment(&self, apparent_wind: &WindCondition) -> SpatialVector {
        SpatialVector::new(
            self.heel_moment(apparent_wind),
            0.0,
            self.yaw_moment(apparent_wind)
        )
    }

    pub fn resistance(&self, apparent_wind: &WindCondition) -> Float {
        let denominator = self.coupling_denominator(apparent_wind.direction_coming_from);

        let force_coefficient = self.resistance_coefficient * apparent_wind.direction_coming_from.cos() / denominator;

        force_coefficient * self.frontal_area * self.dynamic_pressure(apparent_wind.velocity)
    }

    pub fn side_force(&self, apparent_wind: &WindCondition) -> Float {
        let denominator = self.coupling_denominator(apparent_wind.direction_coming_from);

        let force_coefficient = self.side_force_coefficient * apparent_wind.direction_coming_from.sin() / denominator;

        force_coefficient * self.side_area * self.dynamic_pressure(apparent_wind.velocity)
    }

    fn non_dim_yaw_moment_arm(&self, wind_direction: Float) -> Float {
        self.non_dim_side_area_centroid + self.non_dim_yaw_arm_correction * (wind_direction - PI * 0.5)
    }

    pub fn yaw_moment(&self, apparent_wind: &WindCondition) -> Float {
        let side_force = self.side_force(apparent_wind);

        let arm = self.non_dim_yaw_moment_arm(apparent_wind.direction_coming_from) * self.ship_length;

        arm * side_force
    }

    pub fn heel_moment(&self, apparent_wind: &WindCondition) -> Float {
        let side_force = self.side_force(apparent_wind);

        side_force * self.non_dim_heel_arm * self.ship_length
    }

    fn dynamic_pressure(&self, wind_velocity: Float) -> Float {
        0.5 * self.density * wind_velocity.powf(2.0)
    }

    fn coupling_denominator(&self, wind_direction: Float) -> Float {
        let force_ratio = (self.resistance_coefficient * self.frontal_area) / (self.side_force_coefficient * self.side_area);

        1.0 - 0.5 * self.coupling_factor * (1.0 - force_ratio) * (2.0 * wind_direction).sin().powf(2.0)
    }
}
