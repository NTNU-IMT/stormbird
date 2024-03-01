// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Functionality to prescribe the circulation distribution based on empirical models. Intended to 
//! be used in cases where a normal simulation becomes unstable, or when there are some physical 
//! effects not possible to capture with a line force model that requires empirical modelling.
//! 
//! The typical case is the simulation of rotor sails. First, both lifting line and actuator line
//! simulations of this sail type tends to become very unstable. Second, most rotor sails have an 
//! end plate that is not possible to capture in a lifting line simulation. 

pub mod shape;

use serde::{Serialize, Deserialize};

use crate::line_force_model::LineForceModel;
use crate::vec3::Vec3;

use shape::PrescribedCirculationShape;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
/// Determines which ends that should have zero circulation
pub enum EndsWithZeroCirculation {
    #[default]
    Both,
    First,
    Last
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Model for prescribing the circulation distribution based on empirical models.
pub struct PrescribedCirculation {
    #[serde(default)]
    pub shape: PrescribedCirculationShape,
    #[serde(default)]
    pub ends_with_zero_circulation: EndsWithZeroCirculation,
}

impl PrescribedCirculation {
    /// Returns values for the shape of the circulation distribution, based on the relative span 
    /// values and the variant specified in the Self.
    pub fn get_values(&self, relative_span_distance: &[f64]) -> Vec<f64> {
        let effective_span_distance = self.effective_span_distance(relative_span_distance);

        self.shape.get_values(&effective_span_distance)
    }

    pub fn effective_span_distance(&self, relative_span_distance: &[f64]) -> Vec<f64> {
        match self.ends_with_zero_circulation {
            EndsWithZeroCirculation::Both => relative_span_distance.to_vec(),
            EndsWithZeroCirculation::First => {
                relative_span_distance.iter().map(
                    |x| {
                        (x - 0.5) / 2.0
                    }
                ).collect()
            },
            EndsWithZeroCirculation::Last => {
                relative_span_distance.iter().map(
                    |x| {
                        (x + 0.5) / 2.0
                    }
                ).collect()
            }
        }
    }
}

impl LineForceModel {
    /// Returns a circulation distribution that is forced to follow a specific distribution where 
    /// magnitude and direction is based on the average quantities for each wing.
    pub fn prescribed_circulation_strength(&self, velocity: &[Vec3]) -> Vec<f64> {
        let effective_velocity = self.prescribed_circulation_shape_velocity(velocity);

        let raw_circulation_strength = self.circulation_strength_raw(&effective_velocity);

        let average_circulation_strength = self.wing_averaged_values(&raw_circulation_strength);

        let relative_span_distance = self.relative_span_distance();

        let prescribed_circulation = self.prescribed_circulation.clone().unwrap();

        let prescribed_circulation_shape = prescribed_circulation.get_values(&relative_span_distance);

        let mut corrected_circulation: Vec<f64> = Vec::with_capacity(raw_circulation_strength.len());

        for i in 0..raw_circulation_strength.len() {
            let wing_index = self.wing_index_from_global(i);

            corrected_circulation.push(
                average_circulation_strength[wing_index] * prescribed_circulation_shape[i]
            );
        }

        corrected_circulation
    }

    /// Correct the raw velocity when using a prescribed circulation shape
    /// 
    /// Current implementation: constant values across the wing, based on average values. 
    /// 
    /// Should be updated to something more sophisticated in the future...
    pub fn prescribed_circulation_shape_velocity(&self, raw_velocity: &[Vec3]) -> Vec<Vec3> {
        let wing_averaged_velocity = self.wing_averaged_values(raw_velocity);

        self.section_values_from_wing_values(&wing_averaged_velocity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::math_utils::integration;
    use crate::math_utils::array_generation;

    #[test]
    /// Test that the prescribed circulation shape is integrated to 1.0, as it should be.
    fn evaluate_integral_value() {
        let x = array_generation::linspace(-0.5, 0.5, 1000);

        let shape = PrescribedCirculationShape::Elliptic;

        let prescribed_circulation = PrescribedCirculation {
            shape,
            ..Default::default()
        };

        let y = prescribed_circulation.get_shape(&x);

        let integral = integration::trapz(&x, &y);

        dbg!(integral);

        assert!((integral - 1.0).abs() < 1e-4);
        
    }
}