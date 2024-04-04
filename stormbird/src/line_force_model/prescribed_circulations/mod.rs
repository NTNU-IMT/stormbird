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
        let wing_averaged_velocity = self.wing_averaged_values(velocity);

        let effective_velocity = self.section_values_from_wing_values(&wing_averaged_velocity);

        let raw_circulation_strength = self.circulation_strength_raw(&effective_velocity);

        let relative_span_distance = self.relative_span_distance();

        let prescribed_circulation = self.prescribed_circulation.clone().unwrap();

        let prescribed_circulation_shape = prescribed_circulation.get_values(&relative_span_distance);
        
        let wing_averaged_prescribed_circulation_shape = self.wing_averaged_values(&prescribed_circulation_shape);

        let mut corrected_circulation: Vec<f64> = Vec::with_capacity(raw_circulation_strength.len());

        for i in 0..raw_circulation_strength.len() {
            let wing_index = self.wing_index_from_global(i);

            corrected_circulation.push(
                raw_circulation_strength[i] * prescribed_circulation_shape[i] / wing_averaged_prescribed_circulation_shape[wing_index]
            );
        }

        corrected_circulation
    }
}