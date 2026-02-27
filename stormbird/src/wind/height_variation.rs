// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use serde::{Deserialize, Serialize};

use stormath::type_aliases::Float;
use stormath::consts::PI;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum HeightVariationModel {
    PowerModel(PowerModel),
    LogarithmicModel(LogarithmicModel),
}

impl Default for HeightVariationModel {
    fn default() -> Self {
        Self::PowerModel(PowerModel::default())
    }
}

impl HeightVariationModel {
    pub fn velocity_increase_factor(&self, height: Float) -> Float {
        match self {
            HeightVariationModel::PowerModel(model) => 
                model.velocity_increase_factor(height),
            HeightVariationModel::LogarithmicModel(model) => 
                model.velocity_increase_factor(height),
        }
    }

    pub fn from_json_string(json_string: &str) -> Self {
        serde_json::from_str(json_string).unwrap()
    }

    pub fn from_json_file(file_path: &str) -> Self {
        let json_string = std::fs::read_to_string(file_path).unwrap();
        Self::from_json_string(&json_string)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PowerModel {
    #[serde(default="PowerModel::default_reference_height")]
    pub reference_height: Float,
    #[serde(default="PowerModel::default_power_factor")]
    pub power_factor: Float,
}

impl PowerModel {
    pub fn default_reference_height() -> Float {10.0}
    pub fn default_power_factor() -> Float {1.0/9.0}

    pub fn velocity_increase_factor(&self, height: Float) -> Float {
        if self.power_factor > 0.0 {
            (height / self.reference_height).powf(self.power_factor)
        } else {
            1.0
        }
    }
}

impl Default for PowerModel {
    fn default() -> Self {
        Self {
            reference_height: Self::default_reference_height(),
            power_factor: Self::default_power_factor(),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
pub enum AtmosphereState {
    #[default]
    Neutral,
    Stable,
    Unstable
}


#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct LogarithmicModel {
    pub friction_velocity: Float,
    pub surface_roughness: Float,
    #[serde(default="LogarithmicModel::default_von_karman_constant")]
    pub von_karman_constant: Float,
    #[serde[default]]
    pub obukhov_length: Option<Float>,
    #[serde(default)]
    pub atmosphere_state: AtmosphereState
}

impl LogarithmicModel {
    pub fn default_von_karman_constant() -> Float {0.41}
    
    pub fn velocity_increase_factor(&self, height: Float) -> Float {
        if self.surface_roughness <= 0.0 {
            return 1.0;
        }
        
        let factor = self.friction_velocity / self.von_karman_constant;
            
        factor * (height / self.friction_velocity).ln()
    }
    
    /// Function that computer the Businger-Dyer correction based on the atmosphere state and 
    /// the Obukhov length
    pub fn businger_dyer_correction(&self, height: Float) -> Float {
        if let Some(length) = self.obukhov_length {
            let zeta = height / length;
            
            match self.atmosphere_state {
                AtmosphereState::Neutral => 0.0,
                AtmosphereState::Unstable => {
                    let x = (1.0 - 16.0 * zeta).powf(1.0/4.0);
                    
                    let first_term = 2.0 * ((1.0 + x) / 2.0).ln();
                    let second_term = 2.0 * ((1.0 + x.powi(2)) / 2.0).ln();
                    let third_term = - 2.0 * x.atan() + PI / 2.0;
                    
                    first_term + second_term + third_term
                },
                AtmosphereState::Stable => {
                    -5.0 * zeta
                }
            }
        } else {
            0.0
        }
    }
}
