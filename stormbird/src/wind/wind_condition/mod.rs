// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Functionality to represent the wind at a given instance. This includes the variables necessary 
//! to represent the "true wind". 

use serde::{Serialize, Deserialize};

use stormath::type_aliases::Float;

pub mod velocity_variation;
pub mod discretized_spectrum;

use velocity_variation::VelocityVariation;
use discretized_spectrum::DiscretizedSpectrum;

#[derive(Debug, Clone, Serialize, Deserialize)]
/// The wind condition defines the true wind parameters for a given instance. This includes:
/// - The direction of the true wind
/// - How the velocity varies with height
/// - Optional gust spectrums parallel, perpendicular and vertically relative to the steady wind
/// 
/// The structure is typically input to methods in [`super::environment::WindEnvironment`]
pub struct WindCondition {
    /// The direction the wind is coming from, measured in radians. The rotational direction (around 
    /// which axis the wind is rotated) is defined in [`crate::wind::environment::WindEnvironment`] 
    pub direction_coming_from: Float,
    /// How the velocity varies with height
    pub velocity_variation: VelocityVariation,
    #[serde(default)]
    /// Optional gust spectrum for the velocity component parallel to the true wind
    pub parallel_gust: Option<DiscretizedSpectrum>,
    #[serde(default)]
    /// Optional gust spectrum for the velocity component perpendicular to the true wind
    pub perpendicular_gust: Option<DiscretizedSpectrum>,
    #[serde(default)]
    /// Optional gust spectrum for the velocity component vertical to the true wind
    pub vertical_gust: Option<DiscretizedSpectrum>
}

impl WindCondition {
    pub fn new_constant(direction_coming_from: Float, velocity: Float) -> Self {
        Self{
            direction_coming_from,
            velocity_variation: VelocityVariation::Constant(velocity),
            parallel_gust: None,
            perpendicular_gust: None,
            vertical_gust: None
        }
    }
    
    /// Returns the true velocity magnitude at the height gives and input. 
    pub fn steady_true_wind_velocity_at_height(&self, height: Float) -> Float {
        self.velocity_variation.true_wind_velocity_at_height(height)
    }

    /// Unsteady velocity component parallel to the true wind
    pub fn unsteady_parallel_true_wind_velocity_at_height(&self, height: Float, time: Float) -> Float {
        let mut u = self.velocity_variation.true_wind_velocity_at_height(height);
        
        if let Some(spectrum) = &self.parallel_gust {
            u += spectrum.value_at_time(time);
        }
        
        u
    }

    /// Unsteady velocity component perpendicular to the true wind
    pub fn unsteady_perpendicular_true_wind_velocity(&self, time: Float) -> Float {
        if let Some(spectrum) = &self.perpendicular_gust {
            spectrum.value_at_time(time)
        } else {
            0.0
        }
    }
    
    /// Unsteady velocity component vertically to the true wind
    pub fn unsteady_vertical_true_wind_velocity(&self, time: Float) -> Float {
        if let Some(spectrum) = &self.vertical_gust {
            spectrum.value_at_time(time)
        } else {
            0.0
        }
    }

    /// Set parallel gust spectrum from JSON input
    pub fn set_parallel_gust_from_json_string(&mut self, gust_string: &str) {
        let parallel_gust: DiscretizedSpectrum = serde_json::from_str(gust_string).unwrap();
        
        self.parallel_gust = Some(parallel_gust);
    }

    /// Set perpendicular gust spectrum from JSON input
    pub fn set_perpendicular_gust_from_json_string(&mut self, gust_string: &str) {
        let perpendicular_gust: DiscretizedSpectrum = serde_json::from_str(&gust_string).unwrap();
        
        self.perpendicular_gust = Some(perpendicular_gust);
    }

    /// Set vertical gust spectrum from JSON input
    pub fn set_vertical_gust_from_json_string(&mut self, gust_string: &str) {
        let vertical_gust: DiscretizedSpectrum = serde_json::from_str(gust_string).unwrap();
        
        self.vertical_gust = Some(vertical_gust);
    }
}
