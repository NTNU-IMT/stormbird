// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use stormath::{
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
/// Forces are based on the "Blendermann model". The moment formulation is slightly simplified, and 
/// computed from the forces and the specified center of effort.
/// Source: W. Blendermann, 1994, Parameter identification of wind loads on ships,
/// Journal of Wind Engineering and Industrial Aerodynamics, 51, 339-351.
/// Link: https://www.sciencedirect.com/science/article/pii/0167610594900671
pub struct BlendermannSuperstructureForces {
    pub frontal_area: Float,
    pub side_area: Float,
    pub center_of_effort: SpatialVector,
    #[serde(default="BlendermannSuperstructureForces::default_resistance_coefficient")]
    pub resistance_coefficient: Float,
    #[serde(default="BlendermannSuperstructureForces::default_side_force_coefficient")]
    pub side_force_coefficient: Float,
    #[serde(default="BlendermannSuperstructureForces::default_coupling_factor")]
    pub coupling_factor: Float,
    #[serde(default="BlendermannSuperstructureForces::default_air_density")]
    pub density: Float,
}

impl BlendermannSuperstructureForces {
    fn default_resistance_coefficient() -> Float {0.55}
    fn default_side_force_coefficient() -> Float {0.85}
    fn default_coupling_factor() -> Float {0.55}
    fn default_air_density() -> Float {1.225}

    pub fn from_json_string(json_string: &str) -> Result<Self, Error> {
        let serde_res = serde_json::from_str(json_string)?;
        
        Ok(serde_res)
    }

    pub fn from_json_file(file_path: &str) -> Result<Self, Error> {
        let json_string = std::fs::read_to_string(file_path)?;
        
        Self::from_json_string(&json_string)
    }

    pub fn body_fixed_force(&self, body_fixed_velocity: SpatialVector) -> SpatialVector {

        let apparent_wind = WindCondition::from_velocity_vector_assuming_ned(
            body_fixed_velocity
        );
        
        SpatialVector::new(
            -self.resistance(&apparent_wind),
            -self.side_force(&apparent_wind),
            0.0
        )
    }

    pub fn body_fixed_moment(&self, body_fixed_force: SpatialVector) -> SpatialVector {
        let sway_force = body_fixed_force[1];

        SpatialVector::new(
            -sway_force * self.center_of_effort[2],
            0.0,
            sway_force * self.center_of_effort[0]
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

    fn dynamic_pressure(&self, wind_velocity: Float) -> Float {
        0.5 * self.density * wind_velocity.powf(2.0)
    }

    fn coupling_denominator(&self, wind_direction: Float) -> Float {
        let force_ratio = (self.resistance_coefficient * self.frontal_area) / (self.side_force_coefficient * self.side_area);

        1.0 - 0.5 * self.coupling_factor * (1.0 - force_ratio) * (2.0 * wind_direction).sin().powf(2.0)
    }
}
