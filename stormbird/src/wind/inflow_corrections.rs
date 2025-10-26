// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)


use serde::{Serialize, Deserialize};

use stormath::type_aliases::Float;
use stormath::spatial_vector::SpatialVector;
use stormath::interpolation::linear_interpolation;

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Structure that stores inflow correction data for a single sail and a single apparent wind
/// direction, as a function of non-dimensional span distance. The correction data includes a *wake*
/// factor magnitude and an angle correction. The wake factor magnitude is used to reduce the
/// velocity magnitude, while the angle correction is used to rotate the velocity vector around
/// the up direction.
pub struct InflowCorrectionSingleSailSingleDirection {
    pub non_dimensional_span_distances: Vec<Float>,
    pub wake_factors_magnitude: Vec<Float>,
    pub angle_corrections: Vec<Float>,
}

impl InflowCorrectionSingleSailSingleDirection {
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
/// Structure that stores inflow correction data for a single sail, as a function of apparent wind
/// direction. For each apparent wind direction, a set of correction data as a function of
/// non-dimensional span distance is stored.
pub struct InflowCorrectionSingleSail {
    pub apparent_wind_directions: Vec<Float>,
    pub corrections: Vec<InflowCorrectionSingleSailSingleDirection>,
}

impl InflowCorrectionSingleSail {
    pub fn correct_velocity(
        &self,
        apparent_wind_direction: Float,
        non_dimensional_span_distance: Float,
        velocity: SpatialVector,
        up_direction: SpatialVector
    ) -> SpatialVector {
        let corrected_velocities: Vec<SpatialVector> = self.corrections.iter()
            .map(|correction| {
                correction.correct_velocity(
                    non_dimensional_span_distance,
                    velocity,
                    up_direction
                )
            }).collect();

        linear_interpolation(
            apparent_wind_direction,
            &self.apparent_wind_directions,
            &corrected_velocities
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
        apparent_wind_direction: Float,
        non_dimensional_span_distance: Float,
        velocity: SpatialVector,
        up_direction: SpatialVector
    ) -> SpatialVector {
        self.individual_corrections[wing_index]
            .correct_velocity(
                apparent_wind_direction,
                non_dimensional_span_distance,
                velocity,
                up_direction
            )
    }
}
