// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use serde::{Deserialize, Serialize};

use stormath::type_aliases::Float;

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

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct LogarithmicModel {
    #[serde(default="LogarithmicModel::default_reference_height")]
    pub reference_height: Float,
    #[serde(default="LogarithmicModel::surface_roughness")]
    pub surface_roughness: Float,
}

impl LogarithmicModel {
    pub fn default_reference_height() -> Float {10.0}
    pub fn surface_roughness() -> Float {0.0002}

    pub fn velocity_increase_factor(&self, height: Float) -> Float {
        if self.surface_roughness > 0.0 {
            (height / self.surface_roughness).ln() / 
            (self.reference_height / self.surface_roughness).ln()
        } else {
            1.0
        }
    }
}