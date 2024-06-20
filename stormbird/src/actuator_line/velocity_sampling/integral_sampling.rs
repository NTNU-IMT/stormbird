// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use crate::vec3::Vec3;

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IntegralVelocitySamplingBuilder {
    pub span_gaussian_width_factor: f64,
}

impl Default for IntegralVelocitySamplingBuilder {
    fn default() -> Self {
        Self {
            span_gaussian_width_factor: 0.5,
        }
    }
}

impl IntegralVelocitySamplingBuilder {
    pub fn build(&self, number_of_points: usize) -> IntegralVelocitySampling {
        IntegralVelocitySampling {
            span_gaussian_width_factor: self.span_gaussian_width_factor,
            integral_numerators: vec![Vec3::default(); number_of_points],
            integral_denominators: vec![0.0; number_of_points],
        }
    }
}

#[derive(Debug, Clone)]
pub struct IntegralVelocitySampling {
    pub span_gaussian_width_factor: f64,
    pub integral_numerators: Vec<Vec3>,
    pub integral_denominators: Vec<f64>,
}

impl IntegralVelocitySampling {
    /// Reset both the numerator_integral and denominator_integral to their default values, to make
    /// it ready for the next iteration.
    pub fn reset(&mut self) {
        for i in 0..self.integral_denominators.len() {
            self.integral_numerators[i] = Vec3::default();
            self.integral_denominators[i] = 0.0;
        }
    }

    /// Calculate the velocity at the ctrl points from the stored values in the numerator_integral 
    /// and denominator_integral.
    pub fn ctrl_points_velocity(&self) -> Vec<Vec3> {
        let mut velocities = Vec::new();

        for i in 0..self.integral_denominators.len() {
            if self.integral_denominators[i] == 0.0 {
                velocities.push(Vec3::default());
            } else {
                velocities.push(self.integral_numerators[i] / self.integral_denominators[i]);
            }
        }

        velocities
    }

    pub fn add_cell_information(&mut self, line_index: usize, velocity: Vec3, volume: f64, projection_value: f64) {
        let denominator = volume * projection_value;

        self.integral_numerators[line_index] += velocity * denominator;
        self.integral_denominators[line_index] += denominator;
    }
}