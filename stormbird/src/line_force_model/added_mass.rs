// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Model for the added mass forces on a wing.

use serde::{Serialize, Deserialize};

use stormath::type_aliases::Float;
use stormath::consts::PI;
use stormath::spatial_vector::SpatialVector;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Strip theory model of the added mass forces on a single wing.
///
/// The added mass of each line element is given as coefficients times a reference mass, which is
/// the added mass of a flat plate accelerating normal to its chord in two-dimensional potential
/// flow:
///
/// m_ref = density * pi * chord_length^2 / 4 * span_length
///
/// The acceleration normal to the span line is divided in a part along the chord and a part normal
/// to the chord, and each part has its own coefficient. For a rotating cylinder, the chord length
/// is the diameter. Both coefficients are zero by default, and the model is therefore not used.
pub struct AddedMass {
    #[serde(default)]
    /// Added mass coefficient for acceleration normal to the chord. The theoretical value for
    /// both a thin foil and a circular cylinder is 1.0.
    pub normal_coefficient: Float,
    #[serde(default)]
    /// Added mass coefficient for acceleration along the chord. The theoretical value for an
    /// elliptical section is the thickness ratio squared, i.e., close to zero for thin foils, and
    /// 1.0 for a circular cylinder.
    pub chordwise_coefficient: Float,
}

impl AddedMass {
    pub fn is_active(&self) -> bool {
        self.normal_coefficient != 0.0 || self.chordwise_coefficient != 0.0
    }

    /// Calculates the added mass force on a single line element.
    ///
    /// # Arguments
    /// * `density` - the density of the fluid
    /// * `chord_vector` - the chord vector of the line element
    /// * `span_length` - the length of the line element
    /// * `acceleration` - the felt acceleration of the flow at the control point, with the
    /// component along the span line removed
    pub fn force(
        &self,
        density: Float,
        chord_vector: SpatialVector,
        span_length: Float,
        acceleration: SpatialVector,
    ) -> SpatialVector {
        let chord_length = chord_vector.length();

        if chord_length == 0.0 {
            return SpatialVector::default();
        }

        let reference_mass = density * PI * chord_length.powi(2) / 4.0 * span_length;

        let chordwise_acceleration = acceleration.project(chord_vector);
        let normal_acceleration = acceleration - chordwise_acceleration;

        reference_mass * (
            self.normal_coefficient * normal_acceleration +
            self.chordwise_coefficient * chordwise_acceleration
        )
    }
}
