use serde::{Serialize, Deserialize};

use stormath::spatial_vector::SpatialVector;
use stormath::interpolation::linear_interpolation;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InflowCorrectionSingleSail {
    pub non_dimensional_span_distances: Vec<f64>,
    pub wake_factors_magnitude: Vec<f64>,
    pub angle_corrections: Vec<f64>,
}

impl InflowCorrectionSingleSail {
    pub fn correct_velocity(
        &self,
        non_dimensional_span_distance: f64,
        velocity: SpatialVector<3>, 
        up_vector: SpatialVector<3>
    ) -> SpatialVector<3> {
        let wake_factor_magnitude = linear_interpolation(
            non_dimensional_span_distance, 
            &self.non_dimensional_span_distances, 
            &self.wake_factors_magnitude
        );

        let angle_correction = linear_interpolation(
            non_dimensional_span_distance, 
            &self.non_dimensional_span_distances, 
            &self.angle_corrections
        );

        (velocity * (1.0 - wake_factor_magnitude)).rotate_around_axis(
            angle_correction, 
            up_vector
        )

    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InflowCorrections {
    pub corrections_for_each_sail: Vec<InflowCorrectionSingleSail>,
}