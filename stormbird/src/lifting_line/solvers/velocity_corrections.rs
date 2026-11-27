// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Settings for how the induced velocities from the wake models are used in the solvers, including
//! empirical corrections of the velocities.

use serde::{Serialize, Deserialize};

use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

use crate::lifting_line::wake::frozen_wake::InducedVelocities;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Settings for how the induced velocities at the control points are combined with the freestream
/// velocity in the solvers.
///
/// The induced velocities are computed in two parts: the velocities induced by the wake of the same
/// wing as the control point (self-induced), and the velocities induced by the wakes of the other
/// wings (other-induced). The velocity at a control point is then computed in the following order:
/// 1. The self-induced velocity is set to zero if `neglect_self_induced_velocities` is true, and
///    otherwise corrected with the `self_induced_correction`.
/// 2. The induced velocity is the sum of the self-induced velocity from step 1 and the
///    other-induced velocity.
/// 3. The `total_correction` is applied, together with the freestream velocity.
///
/// The corrections are empirical, and the main use case is to stabilize and correct simulations
/// where the lift-induced velocities gets too high, such as for rotor sails.
pub struct InducedVelocitySettings {
    /// Option to neglect the induced velocities on a wing from the wake of the same wing. This is
    /// useful if the effect of self-induced velocities on lift and drag is calculated in another
    /// way, for example with CFD, and the only reason for running lifting-line simulations is to
    /// calculate the wing-to-wing interaction.
    ///
    /// **WARNING**: should probably always be used in combination with a prescribed circulation
    /// shape in the line force model to maintain a realistic local shape.
    #[serde(default)]
    pub neglect_self_induced_velocities: bool,
    /// Correction of the self-induced velocities only
    #[serde(default)]
    pub self_induced_correction: SelfInducedVelocityCorrection,
    /// Correction of the total velocity, i.e., after the self-induced and the other-induced
    /// velocities are added together
    #[serde(default)]
    pub total_correction: TotalVelocityCorrection,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Empirical correction of the self-induced velocities
pub enum SelfInducedVelocityCorrection {
    /// No correction
    #[default]
    NoCorrection,
    /// The magnitude of the self-induced velocity is limited to the given ratio of the magnitude of
    /// the freestream velocity
    MaxInducedVelocityMagnitudeRatio(Float),
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Empirical correction of the total velocity
pub enum TotalVelocityCorrection {
    /// No correction
    #[default]
    NoCorrection,
    /// The magnitude of the total induced velocity is limited to the given ratio of the magnitude
    /// of the freestream velocity
    MaxInducedVelocityMagnitudeRatio(Float),
    /// The total velocity is kept at a fixed magnitude, equal to the freestream velocity
    FixedMagnitudeEqualToFreestream,
}

impl InducedVelocitySettings {
    /// Returns the velocity at the control points, from the freestream velocity and the induced
    /// velocities, following the steps in the documentation of [`InducedVelocitySettings`].
    pub fn ctrl_points_velocity(
        &self,
        freestream_velocities: &[SpatialVector],
        induced_velocities: &InducedVelocities,
    ) -> Vec<SpatialVector> {
        freestream_velocities.iter().enumerate().map(|(i, &freestream_velocity)| {
            let self_induced = if self.neglect_self_induced_velocities {
                SpatialVector::default()
            } else {
                self.self_induced_correction.corrected_induced_velocity(
                    freestream_velocity,
                    induced_velocities.self_induced[i]
                )
            };

            let induced_velocity = self_induced + induced_velocities.other_induced[i];

            self.total_correction.corrected_velocity(freestream_velocity, induced_velocity)
        }).collect()
    }

    /// Returns the induced velocity at a control point that does not depend on the circulation of
    /// the first wake panels, without any corrections. That is, the other-induced velocity, plus the
    /// self-induced velocity if it is not neglected. Used by the linearized solver.
    pub fn uncorrected_induced_velocity(
        &self,
        self_induced: SpatialVector,
        other_induced: SpatialVector,
    ) -> SpatialVector {
        if self.neglect_self_induced_velocities {
            other_induced
        } else {
            self_induced + other_induced
        }
    }
}

/// Limits the magnitude of the induced velocity to the ratio times the magnitude of the freestream
fn limit_induced_velocity_magnitude(
    ratio: Float,
    freestream_velocity: SpatialVector,
    induced_velocity: SpatialVector,
) -> SpatialVector {
    let induced_velocity_magnitude = induced_velocity.length();
    let max_induced_velocity_magnitude = ratio * freestream_velocity.length();

    if induced_velocity_magnitude > max_induced_velocity_magnitude {
        induced_velocity.normalize() * max_induced_velocity_magnitude
    } else {
        induced_velocity
    }
}

impl SelfInducedVelocityCorrection {
    /// Returns the corrected self-induced velocity
    pub fn corrected_induced_velocity(
        &self,
        freestream_velocity: SpatialVector,
        induced_velocity: SpatialVector,
    ) -> SpatialVector {
        match self {
            Self::NoCorrection => induced_velocity,
            Self::MaxInducedVelocityMagnitudeRatio(ratio) => {
                limit_induced_velocity_magnitude(*ratio, freestream_velocity, induced_velocity)
            },
        }
    }
}

impl TotalVelocityCorrection {
    /// Returns the corrected total velocity, i.e., the freestream velocity plus the corrected
    /// induced velocity
    pub fn corrected_velocity(
        &self,
        freestream_velocity: SpatialVector,
        induced_velocity: SpatialVector,
    ) -> SpatialVector {
        match self {
            Self::NoCorrection => freestream_velocity + induced_velocity,
            Self::MaxInducedVelocityMagnitudeRatio(ratio) => {
                freestream_velocity + limit_induced_velocity_magnitude(
                    *ratio,
                    freestream_velocity,
                    induced_velocity
                )
            },
            Self::FixedMagnitudeEqualToFreestream => {
                (freestream_velocity + induced_velocity).normalize() * freestream_velocity.length()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn induced_velocities() -> InducedVelocities {
        InducedVelocities {
            self_induced: vec![SpatialVector::from([0.0, 3.0, 0.0])],
            other_induced: vec![SpatialVector::from([0.0, 0.0, 1.0])],
        }
    }

    const FREESTREAM: [Float; 3] = [2.0, 0.0, 0.0];

    #[test]
    fn no_correction_adds_all_parts() {
        let velocity = InducedVelocitySettings::default().ctrl_points_velocity(
            &[SpatialVector::from(FREESTREAM)],
            &induced_velocities()
        );

        assert_eq!(velocity[0], SpatialVector::from([2.0, 3.0, 1.0]));
    }

    #[test]
    fn neglect_self_induced_velocities_only_keeps_other_induced() {
        let settings = InducedVelocitySettings {
            neglect_self_induced_velocities: true,
            ..Default::default()
        };

        let velocity = settings.ctrl_points_velocity(
            &[SpatialVector::from(FREESTREAM)],
            &induced_velocities()
        );

        assert_eq!(velocity[0], SpatialVector::from([2.0, 0.0, 1.0]));
    }

    #[test]
    fn self_induced_correction_only_limits_self_induced() {
        let settings = InducedVelocitySettings {
            self_induced_correction: SelfInducedVelocityCorrection::MaxInducedVelocityMagnitudeRatio(0.5),
            ..Default::default()
        };

        let velocity = settings.ctrl_points_velocity(
            &[SpatialVector::from(FREESTREAM)],
            &induced_velocities()
        );

        // The self-induced velocity is limited to 0.5 * 2.0 = 1.0, while the other-induced is kept
        assert!((velocity[0] - SpatialVector::from([2.0, 1.0, 1.0])).length() < 1e-12);
    }

    #[test]
    fn total_correction_limits_the_sum() {
        let settings = InducedVelocitySettings {
            total_correction: TotalVelocityCorrection::MaxInducedVelocityMagnitudeRatio(0.5),
            ..Default::default()
        };

        let velocity = settings.ctrl_points_velocity(
            &[SpatialVector::from(FREESTREAM)],
            &induced_velocities()
        );

        let induced = velocity[0] - SpatialVector::from(FREESTREAM);
        let direction = SpatialVector::from([0.0, 3.0, 1.0]).normalize();

        assert!((induced - direction).length() < 1e-12);
    }

    #[test]
    fn fixed_magnitude_keeps_the_freestream_magnitude() {
        let settings = InducedVelocitySettings {
            total_correction: TotalVelocityCorrection::FixedMagnitudeEqualToFreestream,
            ..Default::default()
        };

        let velocity = settings.ctrl_points_velocity(
            &[SpatialVector::from(FREESTREAM)],
            &induced_velocities()
        );

        let expected = SpatialVector::from([2.0, 3.0, 1.0]).normalize() * 2.0;

        assert!((velocity[0] - expected).length() < 1e-12);
    }
}
