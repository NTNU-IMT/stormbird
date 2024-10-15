use serde::{Serialize, Deserialize};

use crate::line_force_model::{CoordinateSystem, LineForceModel};
use math_utils::spatial_vector::SpatialVector;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Parametric model used to calculate the circulation distribution 
pub struct PrescribedCirculationShape {
    #[serde(default = "PrescribedCirculationShape::default_inner_power")]
    pub inner_power: f64,
    #[serde(default = "PrescribedCirculationShape::default_outer_power")]
    pub outer_power: f64,
}

impl Default for PrescribedCirculationShape {
    /// Default corresponds to an elliptical circulation distribution
    fn default() -> Self {
        PrescribedCirculationShape {
            inner_power: PrescribedCirculationShape::default_inner_power(),
            outer_power: PrescribedCirculationShape::default_outer_power(),
        }
    }
}

impl PrescribedCirculationShape {
    pub fn default_inner_power() -> f64 {2.0}
    pub fn default_outer_power() -> f64 {0.5}

    /// Returns the circulation distribution based on the relative span distance
    pub fn get_values(&self, effective_relative_span_distance: &[f64]) -> Vec<f64> {
        let mut values = vec![0.0; effective_relative_span_distance.len()];

        for (i, x) in effective_relative_span_distance.iter().enumerate() {
            if x.abs() < 0.5 {
                values[i] = (1.0 - (2.0 * x.abs()).powf(self.inner_power)).powf(self.outer_power);
            }
        }

        values
    }
}

impl LineForceModel {
    fn effective_span_distance_for_prescribed_circulations(&self) -> Vec<f64> {
        let relative_span_distance = self.relative_span_distance();

        relative_span_distance.iter().enumerate().map(
            |(index, value)| {
                let wing_index = self.wing_index_from_global(index);
                    match self.non_zero_circulation_at_ends[wing_index] {
                        [true, true] => *value, // TODO: consider if this case should behave differently. Not clear how it should be handled....
                        [true, false] => (value + 0.5) / 2.0,
                        [false, true] => (value - 0.5) / 2.0,
                        [false, false] => *value
                    }
            }
        ).collect()
    }

    /// Returns a circulation distribution that is forced to follow a specific distribution where 
    /// magnitude and direction is based on the average quantities for each wing.
    pub fn prescribed_circulation_strength(&self, velocity: &[SpatialVector<3>], prescribed_circulation: &PrescribedCirculationShape, input_coordinate_system: CoordinateSystem) -> Vec<f64> {
        let wing_averaged_velocity = self.wing_averaged_values(velocity);

        let effective_velocity = self.section_values_from_wing_values(&wing_averaged_velocity);

        let raw_circulation_strength = self.circulation_strength_raw(&effective_velocity, input_coordinate_system);

        let effective_span_distance = self.effective_span_distance_for_prescribed_circulations();

        let prescribed_circulation_non_scaled = prescribed_circulation.get_values(&effective_span_distance);
        
        let wing_averaged_prescribed_circulation_non_scaled = self.wing_averaged_values(&prescribed_circulation_non_scaled);

        prescribed_circulation_non_scaled.iter().enumerate().map(
            |(i, value)| {
                let wing_index = self.wing_index_from_global(i);

                let prescribed_circulation_shape = value / wing_averaged_prescribed_circulation_non_scaled[wing_index];

                raw_circulation_strength[i] * prescribed_circulation_shape
            }
        ).collect()
    }
}

