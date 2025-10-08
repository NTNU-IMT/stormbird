// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Corrections that can be applied to induced velocities calculated from wake models.

use serde::{Serialize, Deserialize};

use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
/// Model to empirically correct the estimated velocity in lifting line simulations, based on the 
/// fields in this struct.
///
/// The main use case is to stabilize and correct simulations where the lift-induced velocities gets
/// too high. The main use case for this is correcting rotor sail simulations.
pub enum VelocityCorrections {
    #[default]
    None,
    /// The induced velocity is capped using a ratio between the induced velocity and the freestream
    MaxInducedVelocityMagnitudeRatio(Float),
    /// The total velocity is kept at a fixed magnitude, equal to the freestream velocity 
    FixedMagnitudeEqualToFreestream,
}

impl VelocityCorrections {
    pub fn max_induced_velocity_magnitude_ratio(
        ratio: Float,
        freestream_velocities: &[SpatialVector],
        induced_velocities: &[SpatialVector],
    ) -> Vec<SpatialVector> {
        let u_i_corrected: Vec<SpatialVector> = induced_velocities.iter().zip(freestream_velocities.iter()).map(
            |(induced_velocity, freestream_velocity)| {
                let induced_velocity_magnitude = induced_velocity.length();
                let freestream_velocity_magnitude = freestream_velocity.length();

                let max_induced_velocity_magnitude = ratio * freestream_velocity_magnitude;

                if induced_velocity_magnitude > max_induced_velocity_magnitude {
                    induced_velocity.normalize() * max_induced_velocity_magnitude
                } else {
                    induced_velocity.clone()
                }
            }
        ).collect();

        u_i_corrected.iter().zip(freestream_velocities.iter()).map(
            |(induced_velocity, freestream_velocity)| {
                *freestream_velocity + *induced_velocity
            }
        ).collect()
    }

    pub fn fixed_magnitude_equal_to_freestream(
        freestream_velocities: &[SpatialVector],
        induced_velocities: &[SpatialVector],
    ) -> Vec<SpatialVector> {
        let mut u_total: Vec<SpatialVector> = induced_velocities.iter().zip(freestream_velocities.iter()).map(
            |(induced_velocity, freestream_velocity)| {
                *freestream_velocity + *induced_velocity
            }
        ).collect();


        u_total.iter_mut().zip(freestream_velocities.iter()).for_each(
            |(total_velocity, freestream_velocity)| {
                *total_velocity = total_velocity.normalize() * freestream_velocity.length();
            }
        );

        u_total
    }
}