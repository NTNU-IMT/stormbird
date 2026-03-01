// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use serde::{Deserialize, Serialize};

use stormath::type_aliases::Float;

pub mod power_model;
pub mod logarithmic_model;

use power_model::PowerModel;
use logarithmic_model::LogarithmicModel;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
/// Structure used to represent how the velocity magnitude might change as a function of height.
pub enum VelocityVariation {
    /// Variant representing no variation. That is, constant velocity at all heights
    Constant(Float),
    /// The velocity varies according to the power law model for the atmospheric boundary layer.
    /// This is the simplest way to represent a height variation.
    PowerModel(PowerModel),
    /// The velocity variation is set according to a full logarithmic model, which may contain
    /// models for a non-neutral atmosphere.
    LogarithmicModel(LogarithmicModel),
}

impl VelocityVariation {
    pub fn true_wind_velocity_at_height(&self, height: Float) -> Float {
        match self {
            Self::Constant(value) => *value,
            Self::PowerModel(model) => model.velocity_at_height(height),
            Self::LogarithmicModel(model) => model.velocity_at_height(height)
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







