use serde::{Serialize, Deserialize};

use stormath::type_aliases::Float;
use stormath::spatial_vector::SpatialVector;
use stormath::interpolation::linear_interpolation;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InflowCorrectionSingleSail {
    pub non_dimensional_span_distances: Vec<Float>,
    pub wake_factors_magnitude: Vec<Float>,
    pub angle_corrections: Vec<Float>,
}

impl InflowCorrectionSingleSail {
    pub fn correct_velocity(
        &self,
        non_dimensional_span_distance: Float,
        velocity: SpatialVector, 
        up_direction: SpatialVector
    ) -> SpatialVector {
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
            up_direction
        )

    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InflowCorrections {
    pub individual_corrections: Vec<InflowCorrectionSingleSail>,
}

impl InflowCorrections {
    pub fn correct_velocity(
        &self,
        wing_index: usize,
        non_dimensional_span_distance: Float,
        velocity: SpatialVector,
        up_direction: SpatialVector
    ) -> SpatialVector {
        self.individual_corrections[wing_index]
            .correct_velocity(non_dimensional_span_distance, velocity, up_direction)
    }
}