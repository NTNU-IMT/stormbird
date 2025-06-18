use serde::{Serialize, Deserialize};

use crate::line_force_model::{CoordinateSystem, LineForceModel};
use stormath::spatial_vector::SpatialVector;

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
    /// Returns the effective non-dimensional span distance values for each control point.
    pub fn effective_relative_span_distance(&self) -> Vec<f64> {
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

    /// Returns a circulation distribution that is forced to follow the specified distribution 
    /// shape, but where the averaged value still is the same as what is calculated from the raw 
    /// lifting line theory.
    /// 
    /// The shape is enforced on the circulation divided by the square of the velocity,
    /// so that changes due to external velocity variations (e.g., velocity due to rotational 
    /// motion, atmospheric boundary layer, interactions with a another structure, etc.) are taken 
    /// into account.
    /// 
    /// # Arguments
    /// * `velocity` - The velocity at the control points, used to calculate the circulation
    /// * `prescribed_circulation` - The shape of the circulation distribution to be followed
    /// * `input_coordinate_system` - The coordinate system in which the input velocity is given.
    pub fn prescribed_circulation_strength(
        &self, 
        velocity: &[SpatialVector<3>], 
        prescribed_circulation: &PrescribedCirculationShape, 
        input_coordinate_system: CoordinateSystem
    ) -> Vec<f64> {

        let raw_circulation_strength = self.circulation_strength_raw(
            velocity, 
            input_coordinate_system
        );

        let velocity_squared: Vec<f64> = velocity.iter().map(
            |v| v.length_squared()
        ).collect();

        let mut gamma_divided_by_u2 = Vec::with_capacity(raw_circulation_strength.len());

        for i in 0..raw_circulation_strength.len() {
            if velocity_squared[i] < f64::MIN_POSITIVE {
                gamma_divided_by_u2.push(0.0);
            } else {
                gamma_divided_by_u2.push(raw_circulation_strength[i] / velocity_squared[i]);
            }
        }

        let averaged_gamma_divided_by_u2 = self.wing_averaged_values(
            &gamma_divided_by_u2
        );

        let effective_relative_span_distance = self.effective_relative_span_distance();

        let prescribed_circulation_shape = prescribed_circulation.get_values(
            &effective_relative_span_distance
        );

        let averaged_prescribed_circulation_shape = self.wing_averaged_values(
            &prescribed_circulation_shape
        );

        let mut out: Vec<f64> = Vec::with_capacity(raw_circulation_strength.len());

        for i in 0..raw_circulation_strength.len() {
            let wing_index = self.wing_index_from_global(i);

            let factor = if averaged_prescribed_circulation_shape[wing_index] == 0.0 {
                0.0
            } else {
                averaged_gamma_divided_by_u2[wing_index] / 
                averaged_prescribed_circulation_shape[wing_index]
            };

            out.push(
                factor * 
                prescribed_circulation_shape[i] * 
                velocity_squared[i]
            )
        }

        out
    }
}

