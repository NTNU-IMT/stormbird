// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use stormbird::wind::wind_condition::WindCondition as WindConditionRust;

use stormbird::wind::wind_condition::velocity_variation::{
    VelocityVariation,
    power_model::PowerModel,
    logarithmic_model::{
        LogarithmicModel, AtmosphereState
    }
};

use pyo3::prelude::*;
use pyo3::types::PyType;

#[pyclass]
#[derive(Clone)]
pub struct WindCondition {
    pub data: WindConditionRust
}

#[pymethods]
impl WindCondition {
    #[classmethod]
    #[pyo3(signature=(
        *,
        direction_coming_from,
        velocity,
    ))]
    pub fn new_constant(_cls: &Bound<'_, PyType>, direction_coming_from: f64, velocity: f64) -> Self {
        WindCondition{
            data: WindConditionRust{
                direction_coming_from: direction_coming_from,
                velocity_variation: VelocityVariation::Constant(velocity)
            }
        }
    }
    
    #[classmethod]
    #[pyo3(signature=(
        *,
        direction_coming_from,
        reference_velocity,
        reference_height = 10.0,
        power_factor = 0.11111
    ))]
    pub fn new_power_model(
        _cls: &Bound<'_, PyType>, 
        direction_coming_from: f64,
        reference_velocity: f64,
        reference_height: f64,
        power_factor: f64
    ) -> Self {
        WindCondition {
            data: WindConditionRust {
                direction_coming_from: direction_coming_from,
                velocity_variation: VelocityVariation::PowerModel(
                    PowerModel {
                        reference_velocity, 
                        reference_height, 
                        power_factor 
                    }
                )
            }
        }
    }
    
    #[classmethod]
    #[pyo3(signature=(
        *,
        direction_coming_from,
        friction_velocity,
        surface_roughness,
        von_karman_constant = 0.41,
        obukhov_length = 0.0,
    ))]
    pub fn new_logarithmic_model(
        _cls: &Bound<'_, PyType>,
        direction_coming_from: f64,
        friction_velocity: f64,
        surface_roughness: f64,
        von_karman_constant: f64,
        obukhov_length: f64
    ) -> Self {
        let mut obukhov_length_rust = None;
        
        if obukhov_length != 0.0 {
            obukhov_length_rust = Some(obukhov_length);
        }
        
        WindCondition {
            data: WindConditionRust {
                direction_coming_from,
                velocity_variation: VelocityVariation::LogarithmicModel(
                    LogarithmicModel {
                        friction_velocity,
                        surface_roughness,
                        von_karman_constant,
                        obukhov_length: obukhov_length_rust,
                        atmosphere_state: AtmosphereState::Neutral
                    }
                )
            }
        }
    }
    
    pub fn true_wind_velocity_at_height(&self, height: f64) -> f64 {
        self.data.true_wind_velocity_at_height(height)
    }
}