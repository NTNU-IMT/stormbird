// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Model for the gyroscopic moment from a spinning wing, i.e., a rotor sail.

use serde::{Serialize, Deserialize};

use stormath::type_aliases::Float;
use stormath::consts::TAU;
use stormath::spatial_vector::SpatialVector;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Model of the gyroscopic moment from a rotor sail, due to the rotation of the rotor combined
/// with the rotation of the whole line force model, for instance due to roll or pitch motion of a
/// ship.
///
/// The rotational speed of the rotor is assumed to be much larger than the rotational velocity of
/// the line force model. The angular momentum of the rotor is then L = I * omega_rotor * e, where e
/// is the span direction, and the moment from the rotor on its supports is L x omega_model. The
/// moment of inertia is distributed along the span in proportion to the length of each line
/// element. Virtual line elements are not included.
///
/// Only used together with the rotating cylinder section model, where the internal state of the
/// wing is the revolutions per second of the rotor. The moment of inertia is zero by default, and
/// the model is therefore not used.
pub struct GyroscopicMoment {
    #[serde(default)]
    /// The total moment of inertia of the rotor about its axis of rotation, in kg m^2.
    pub moment_of_inertia: Float,
}

impl GyroscopicMoment {
    pub fn is_active(&self) -> bool {
        self.moment_of_inertia != 0.0
    }

    /// Calculates the gyroscopic moment from a part of the rotor.
    ///
    /// # Arguments
    /// * `span_fraction` - the fraction of the rotor span that the part represents
    /// * `revolutions_per_second` - the rotational speed of the rotor
    /// * `span_direction` - the direction of the axis of rotation of the rotor
    /// * `rotation_velocity` - the rotational velocity of the whole line force model
    pub fn moment(
        &self,
        span_fraction: Float,
        revolutions_per_second: Float,
        span_direction: SpatialVector,
        rotation_velocity: SpatialVector,
    ) -> SpatialVector {
        let angular_momentum = span_fraction * self.moment_of_inertia *
            TAU * revolutions_per_second * span_direction;

        angular_momentum.cross(rotation_velocity)
    }
}
