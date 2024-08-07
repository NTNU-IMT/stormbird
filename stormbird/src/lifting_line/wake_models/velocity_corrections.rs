// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Corrections that can be applied to induced velocities calculated from wake models.

use serde::{Serialize, Deserialize};

use math_utils::spatial_vector::SpatialVector;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
/// Builder for a [VelocityCorrections] struct. The only difference between the fields is the
/// `max_magnitude_ratio` in the builder vs `max_magnitude` in the build result.
pub struct VelocityCorrectionsBuilder {
    /// A variable that is used to calculate the max magnitude of the induced velocities as a ratio
    /// of the freestream velocity used when building the wake.
    pub max_magnitude_ratio: Option<f64>,
    /// See the documentation for the same field in the [VelocityCorrections] struct
    pub correction_factor: Option<f64>,
}

impl VelocityCorrectionsBuilder {
    /// Returns a [VelocityCorrections] struct based on the settings in `self`
    pub fn build(&self, freestream: SpatialVector<3>) -> VelocityCorrections {
        let max_magnitude = if let Some(max_magnitude_ratio) = self.max_magnitude_ratio {
            Some(max_magnitude_ratio * freestream.length())
        } else {
            None
        };

        VelocityCorrections {
            max_magnitude,
            correction_factor: self.correction_factor
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
/// Model to empirically correct the induced velocity from wake models, based on the fields in this
/// struct.
///
/// The main use case is to stabilize and correct simulations where the lift-induced velocities gets
/// too high. The main use case for this is correcting rotor sail simulations.
pub struct VelocityCorrections {
    /// Option to limit the magnitude of the induced velocity to a max value. This can be used in 
    /// cases where there appears to be physically incorrect singularities. The  main use case is 
    /// for steady-state simulations of rotor sails.
    pub max_magnitude: Option<f64>,
    /// Option to correct the magnitude of the induced velocities by multiplying them with a 
    /// factor. This is meant for cases where the raw induced velocities are estimated to be too 
    /// large. For instance, if a rotor sail is modeled using a lifting line model, it is impossible
    /// to directly model the presence of an end disk. The correction factor can be used to correct
    /// for this. **Warning**: this only affects the magnitude of the induced velocities, not the
    /// shape of the circulation distribution. In other words, it is a simplified approach. However,
    /// it is expected that a simple correction can be appropriate as long as the sails are not
    /// too close to each other.
    pub correction_factor: Option<f64>,
}

impl VelocityCorrections {
    /// This function applies corrections to the induced velocities calculated directly from a wake
    /// model.
    ///
    /// The max magnitude correction is applied first, then the correction factor.
    ///
    /// # Argument
    /// * `induced_velocities` - a vector containing the raw induced velocities calculated from a
    /// wake model, such as [super::unsteady::UnsteadyWake] or [super::steady::SteadyWake]
    pub fn correct(&self, induced_velocities: &mut [SpatialVector<3>]) {
         self.apply_max_magnitude(induced_velocities);
         self.apply_correction_factor(induced_velocities);
    }

    /// Returns true if any of the corrections in the struct is active or Some.
    pub fn any_active_corrections(&self) -> bool {
        if self.max_magnitude.is_some() || self.correction_factor.is_some() {
            true
        } else {
            false
        }
    }

    /// Applies the max magnitude correction. The magnitude of any velocity in the input vector that
    /// is larger than the max magnitude in the struct will be reduced to the max magnitude
    fn apply_max_magnitude(&self, induced_velocities: &mut [SpatialVector<3>]) {
        if let Some(max_magnitude) = self.max_magnitude {
            for i in 0..induced_velocities.len() {
                if induced_velocities[i].length() > max_magnitude {
                    induced_velocities[i] = induced_velocities[i].normalize() * max_magnitude;
                }
            }
        }
    }

    /// Applies the correction factor. All induced velocities in the input vector will be multiplied
    /// with the correction factor.
    fn apply_correction_factor(&self, induced_velocities: &mut [SpatialVector<3>]) {
        if let Some(correction_factor) = self.correction_factor {
            for i in 0..induced_velocities.len() {
                induced_velocities[i] = correction_factor * induced_velocities[i];
            }
        }
    }
}