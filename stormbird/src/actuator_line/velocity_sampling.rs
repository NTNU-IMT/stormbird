// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use crate::vec3::Vec3;

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VelocitySamplingBuilder {
    pub span_gaussian_width_factor: f64,
}

impl Default for VelocitySamplingBuilder {
    fn default() -> Self {
        Self {
            span_gaussian_width_factor: 0.5,
        }
    }
}

impl VelocitySamplingBuilder {
    pub fn build(&self, number_of_points: usize) -> VelocitySampling {
        VelocitySampling {
            span_gaussian_width_factor: self.span_gaussian_width_factor,
            numerator_integral: vec![Vec3::default(); number_of_points],
            denominator_integral: vec![0.0; number_of_points],
        }
    }
}

#[derive(Debug, Clone)]
pub struct VelocitySampling {
    pub span_gaussian_width_factor: f64,
    pub numerator_integral: Vec<Vec3>,
    pub denominator_integral: Vec<f64>,
}

impl VelocitySampling {
    pub fn reset(&mut self) {
        for i in 0..self.numerator_integral.len() {
            self.numerator_integral[i]   = Vec3::default();
            self.denominator_integral[i] = 0.0;
        }
    }

    pub fn freestream_velocity(&self) -> Vec<Vec3> {
        let mut freestream_velocities = Vec::new();

        for i in 0..self.numerator_integral.len() {
            if self.denominator_integral[i] == 0.0 {
                freestream_velocities.push(
                    Vec3::default()
                );

                println!("Warning: denominator_integral is zero at index {}", i);
            } else {
                freestream_velocities.push(
                    self.numerator_integral[i] / self.denominator_integral[i]
                );
            }
        }

        freestream_velocities
    }

    pub fn add_cell_information(&mut self, line_index: usize, velocity: Vec3, volume: f64, projection_value: f64) {
        if line_index >= self.numerator_integral.len() {
            panic!("line_index out of bounds");
        }

        let denominator = volume * projection_value;

        self.numerator_integral[line_index] += velocity * denominator;
        self.denominator_integral[line_index] += denominator;
    }
}