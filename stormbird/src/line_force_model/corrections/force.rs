// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Empirical corrections of the forces on a wing.

use serde::{Serialize, Deserialize};

use stormath::type_aliases::Float;
use stormath::consts::MIN_POSITIVE;
use stormath::spatial_vector::SpatialVector;

use crate::line_force_model::LineForceModel;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Empirical corrections of the forces on a single wing, applied on top of the forces from the
/// line force model. All corrections are zero by default, and therefore not used.
pub struct ForceCorrection {
    #[serde(default)]
    /// Factor for a correction of the lift-induced drag, which adds a drag coefficient equal to
    /// the factor times the square of the lift coefficient on each line element. The lift
    /// coefficient is computed from the total lift force on the element, i.e., the circulatory
    /// force and the viscous lift. A positive value increases the drag, while a negative value
    /// decreases it.
    pub lift_induced_drag_correction: Float,
}

impl ForceCorrection {
    /// Returns the additional drag coefficient for the given lift coefficient
    pub fn drag_coefficient(&self, lift_coefficient: Float) -> Float {
        self.lift_induced_drag_correction * lift_coefficient.powi(2)
    }
}

impl LineForceModel {
    /// Applies the force corrections of each wing to the sectional forces. Wings without active
    /// corrections are skipped.
    ///
    /// # Arguments
    /// * `circulatory` - the circulatory force on each line element
    /// * `viscous_lift` - the viscous lift force on each line element
    /// * `velocity` - the felt velocity at each control point
    /// * `sectional_drag` - the sectional drag force on each line element, which the drag from the
    ///   lift-induced drag correction is added to
    pub fn apply_force_corrections(
        &self,
        circulatory: &[SpatialVector],
        viscous_lift: &[SpatialVector],
        velocity: &[SpatialVector],
        sectional_drag: &mut [SpatialVector],
    ) {
        for (wing_index, wing_indices) in self.wing_indices.iter().enumerate() {
            let correction = &self.force_corrections[wing_index];

            if correction.lift_induced_drag_correction == 0.0 {
                continue;
            }

            for index in wing_indices.clone() {
                let area = self.chord_lengths[index] * self.span_lines_local[index].length();

                let force_factor = 0.5 * area * self.density * velocity[index].length_squared();

                if force_factor < MIN_POSITIVE {
                    continue;
                }

                let lift_force = circulatory[index] + viscous_lift[index];
                let lift_coefficient = lift_force.length() / force_factor;

                sectional_drag[index] += velocity[index].normalize() *
                    correction.drag_coefficient(lift_coefficient) * force_factor;
            }
        }
    }
}
