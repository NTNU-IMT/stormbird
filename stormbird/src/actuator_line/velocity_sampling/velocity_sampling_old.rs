// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use serde::{Serialize, Deserialize};


use math_utils::array_generation::linspace;

use crate::line_force_model::span_line::SpanLine;
use crate::vec3::Vec3;

use super::projection::Projection;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Model for correcting the sampled velocity, based on empirical data that depends on the local 
/// and lift coefficient
pub struct VelocityCorrection {
    pub lift_coefficients: Vec<f64>,
    pub magnitude_corrections: Vec<f64>,
    pub angle_correction: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Struct that controls how the velocity is sampled in an actuator line simulation.
pub struct VelocitySampling {
    pub nr_chord_samples: usize,
    pub nr_thickness_samples: usize,
    pub max_chord_offset_factor: f64,
    pub max_thickness_offset_factor: f64,
    pub velocity_correction: Option<VelocityCorrection>,
}

impl Default for VelocitySampling {
    fn default() -> Self {
        Self {
            nr_chord_samples: 3,
            nr_thickness_samples: 3,
            max_chord_offset_factor: 0.5,
            max_thickness_offset_factor: 0.5,
            velocity_correction: None,
        }
    }
}

impl VelocitySampling {
    pub fn nr_sampling_points_per_element(&self) -> usize {
        self.nr_chord_samples * self.nr_thickness_samples
    }

    pub fn relative_offset_factors(&self) -> (Vec<f64>, Vec<f64>) {
        let chord_offset_factors     = linspace(-self.max_chord_offset_factor, self.max_chord_offset_factor, self.nr_chord_samples);
        let thickness_offset_factors = linspace(-self.max_thickness_offset_factor, self.max_thickness_offset_factor, self.nr_thickness_samples);

        (chord_offset_factors, thickness_offset_factors)
    }

    /// Returns points where the velcoity should be interpolated to for a single element.
    pub fn sampling_points_for_element(&self, span_line: &SpanLine, chord_vector: Vec3) -> Vec<Vec3> {
        let ctrl_point = span_line.ctrl_point();
        
        if self.nr_chord_samples == 1 && self.nr_thickness_samples == 1 {
            vec![ctrl_point]
        } else {
            let mut points: Vec<Vec3> = Vec::new();

            let chord_direction     = chord_vector.normalize();
            let thickness_direction = span_line.relative_vector().cross(chord_direction).normalize();

            let chord_length    = chord_vector.length();
            
            let (chord_offset_factors, thickness_offset_factors) = self.relative_offset_factors();

            for thickness_offset_factor in &thickness_offset_factors {
                for chord_offset_factor in &chord_offset_factors {
                    let chord_offset_distance     = chord_offset_factor * chord_length;
                    let thickness_offset_distance = thickness_offset_factor * chord_length;

                    let point = ctrl_point + chord_direction * chord_offset_distance + thickness_direction * thickness_offset_distance;

                    points.push(point);
                }
            }

            points
        }
    }

    pub fn ctrl_point_velocity_from_sampling_points_velocity(&self, sampling_points_velocity: &[Vec3], nr_span_lines: usize, projection: &Projection) -> Vec<Vec3> {
        let projection_values = self.projection_distribution_for_single_element(projection);
        
        let mut ctrl_point_velocity: Vec<Vec3> = Vec::new();

        let nr_samling_points_per_element = self.nr_sampling_points_per_element();

        let mut current_sampling_point_index = 0;

        for _ in 0..nr_span_lines {
            let mut velocity_sum =  Vec3::default();
            let mut projection_sum = 0.0;

            for i in 0..nr_samling_points_per_element {
                let current_velocity = sampling_points_velocity[current_sampling_point_index + i];

                velocity_sum += current_velocity.length() * projection_values[i] * current_velocity;
                projection_sum += current_velocity.length() * projection_values[i];
            }

            ctrl_point_velocity.push(
                velocity_sum / projection_sum
            );

            current_sampling_point_index += nr_samling_points_per_element;
        }

        ctrl_point_velocity
    }

    pub fn projection_distribution_for_single_element(&self, projection: &Projection) -> Vec<f64> {
        if self.nr_chord_samples == 1 && self.nr_thickness_samples == 1 {
            vec![1.0]
        } else {
            let span_line = SpanLine{
                start_point: Vec3::new(-0.5, 0.0, 0.0),
                end_point: Vec3::new(0.5, 0.0, 0.0),
            };

            let chord_vector = Vec3::new(1.0, 0.0, 0.0);

            let mut projection_values: Vec<f64> = Vec::with_capacity(self.nr_sampling_points_per_element());

            let (chord_offset_factors, thickness_offset_factors) = self.relative_offset_factors();

            for i_t in 0..self.nr_thickness_samples {
                for i_c in 0..self.nr_chord_samples {
                    let point = Vec3::new(
                        chord_offset_factors[i_c], 
                        thickness_offset_factors[i_t], 
                        0.0
                    );

                    projection_values.push(
                        projection.projection_value_at_point(
                            point, chord_vector, &span_line
                        )
                    );
                }
            }

            projection_values
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::actuator_line::projections::gaussian_two_dim::GaussianTwoDim;

    #[test]
    fn projection_distribution_for_single_element() {
        let sampling = VelocitySampling {
            nr_chord_samples: 3,
            nr_thickness_samples: 3,
            max_offset_factor: 0.5,
        };

        let projection = Projection::GaussianTwoDim(GaussianTwoDim::default());

        let result = sampling.projection_distribution_for_single_element(&projection);

        dbg!(result);
    }

    #[test]
    fn ctrl_point_velocity_from_sampling_points_velocity() {
        let sampling = VelocitySampling {
            nr_chord_samples: 2,
            nr_thickness_samples: 2,
            max_offset_factor: 0.5,
        };

        let projection = Projection::GaussianTwoDim(GaussianTwoDim::default());

        let nr_elements = 2;

        let sampling_points_velocity = vec![
            Vec3::default(); sampling.nr_sampling_points_per_element() * nr_elements
        ];

        let result = sampling.ctrl_point_velocity_from_sampling_points_velocity(
            &sampling_points_velocity, 
            nr_elements, 
            &projection
        );

        dbg!(result);
    }
}