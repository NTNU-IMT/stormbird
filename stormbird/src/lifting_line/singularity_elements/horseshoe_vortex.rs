// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use math_utils::spatial_vector::SpatialVector;
use crate::line_force_model::span_line::SpanLine;

use super::PotentialTheoryModel;
use super::viscous_core_length::ViscousCoreLength;

impl PotentialTheoryModel {
    /// Calculates the induced velocity at a control point from a horseshoe vortex with unit 
    /// strength.
    pub fn induced_velocity_from_horseshoe_with_unit_strength(
        &self,
        ctrl_point: SpatialVector<3>,
        span_line: &SpanLine,
        wake_line_vector: SpatialVector<3>,
    ) -> SpatialVector<3> {

        // Assume the points are ordered in the same way as panel points, only neglect the line 
        // between the end points at the back of the wake.
        let line_list: [[SpatialVector<3>; 2]; 3] = [
            [span_line.start_point, span_line.end_point],
            [span_line.end_point,   span_line.end_point + wake_line_vector],
            [span_line.start_point + wake_line_vector, span_line.start_point],
        ];

        let viscous_core_length = match self.viscous_core_length {
            ViscousCoreLength::Relative(core_length) => {
                ViscousCoreLength::Absolute(core_length * span_line.length())
            },
            ViscousCoreLength::Absolute(core_length) => ViscousCoreLength::Absolute(core_length),
            ViscousCoreLength::NoViscousCore => ViscousCoreLength::NoViscousCore,
        };

        let u_i = line_list.iter().map(
            |line| {
                Self::induced_velocity_from_line_with_unit_strength(
                    line, ctrl_point, viscous_core_length, self.closeness_error
                )
            }
        ).sum();

        let ctrl_point_mirrored = self.symmetry_condition.mirrored_point(ctrl_point);

        if let Some(ctrl_point_mirrored) = ctrl_point_mirrored {
            let u_i_m = line_list.iter().map(
                |line| {
                    Self::induced_velocity_from_line_with_unit_strength(
                        line, ctrl_point_mirrored, viscous_core_length, self.closeness_error
                    )
                }
            ).sum();

            self.symmetry_condition.corrected_velocity(u_i, u_i_m)
        } else {
            u_i
        }
    }
}