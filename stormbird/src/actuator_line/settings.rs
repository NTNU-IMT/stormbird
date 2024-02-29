// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActuatorLineSettings {
    #[serde(default)]
    pub strength_damping: f64,
    #[serde(default)]
    pub remove_span_velocity: bool,
    #[serde(default="ActuatorLineSettings::default_gaussian_mapping_length_factor")]
    pub gaussian_mapping_length_factor: f64,
    #[serde(default="ActuatorLineSettings::default_gaussian_mapping_end_correction")]
    pub gaussian_mapping_end_correction: (bool, bool),
    #[serde(default)]
    pub velocity_aligned_projection: bool,
}

impl ActuatorLineSettings {
    pub fn default_gaussian_mapping_length_factor() -> f64 {0.5}
    pub fn default_gaussian_mapping_end_correction() -> (bool, bool) {(false, false)}
}

impl Default for ActuatorLineSettings {
    fn default() -> Self {
        Self {
            strength_damping: 0.0,
            remove_span_velocity: false,
            gaussian_mapping_length_factor: Self::default_gaussian_mapping_length_factor(),
            gaussian_mapping_end_correction: Self::default_gaussian_mapping_end_correction(),
            velocity_aligned_projection: false,
        }
    }
}