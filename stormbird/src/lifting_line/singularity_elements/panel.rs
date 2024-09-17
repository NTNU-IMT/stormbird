// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use std::f64::consts::PI;

use math_utils::spatial_vector::SpatialVector;
use math_utils::spatial_vector::geometry_functions;

use super::PotentialTheoryModel;

impl PotentialTheoryModel {
    /// Function that calculates the induced velocity from a vortex panel, based on the corner points
    /// given as input
    pub fn induced_velocity_from_panel_with_unit_strength(
        &self, 
        panel_points: &[SpatialVector<3>; 4], 
        ctrl_point: SpatialVector<3>,
        viscous_core_length: f64,
    ) -> SpatialVector<3> {
        let panel_center = 0.25 * (panel_points[0] + panel_points[1] + panel_points[2] + panel_points[3]);
        let panel_area = geometry_functions::area_of_quadrilateral(&panel_points);
        let panel_normal = geometry_functions::normal_of_quadrilateral(&panel_points);

        let distance_to_ctrl_point = (ctrl_point - panel_center).length();

        let bound_vortex_length = (panel_points[1] - panel_points[0]).length();

        let stream_length = (panel_points[2] - panel_points[1]).length();

        let representative_length = bound_vortex_length.max(stream_length);

        let u_i = if distance_to_ctrl_point > representative_length * self.far_field_ratio {

            self.induced_velocity_from_panel_as_point_doublet_with_unit_strength(
                panel_area, 
                panel_normal,
                panel_center,
                ctrl_point
            )
        } else {
            self.induced_velocity_from_panel_as_vortex_lines_with_unit_strength(
                panel_points, 
                ctrl_point, 
                viscous_core_length
            )
        };

        let ctrl_point_mirrored = self.symmetry_condition.mirrored_point(ctrl_point);

        if let Some(ctrl_point_mirrored) = ctrl_point_mirrored {
            let mirrored_distance_to_ctrl_point = (ctrl_point_mirrored - panel_center).length();

            let u_i_m = if mirrored_distance_to_ctrl_point > representative_length * self.far_field_ratio {
                self.induced_velocity_from_panel_as_point_doublet_with_unit_strength(
                    panel_area, 
                    panel_normal,
                    panel_center, 
                    ctrl_point_mirrored
                )
            } else {
                self.induced_velocity_from_panel_as_vortex_lines_with_unit_strength(
                    panel_points, 
                    ctrl_point_mirrored, 
                    viscous_core_length
                )
            };

            self.symmetry_condition.corrected_velocity(u_i, u_i_m)
        } else {
            u_i
        }
    }

    pub fn induced_velocity_from_panel_as_vortex_lines_with_unit_strength(
        &self, 
        panel_points: &[SpatialVector<3>; 4], 
        ctrl_point: SpatialVector<3>,
        viscous_core_length: f64,
    ) -> SpatialVector<3> {
        let mut u_i: SpatialVector<3> = SpatialVector::<3>::default();

        for i_point in 0..panel_points.len() {
            let line_points: [SpatialVector<3>; 2] = if i_point == panel_points.len() - 1 {
                [panel_points[i_point], panel_points[0]]
            } else {
                [panel_points[i_point], panel_points[i_point + 1]]
            };

            u_i += Self::induced_velocity_from_line_with_unit_strength(
                &line_points, ctrl_point, viscous_core_length, self.closeness_error
            );
        }

        u_i
    }

    /// Simplified formulation for a panel, based on a point formulation.
    /// Based on the equations from: 
    /// <https://ntrs.nasa.gov/api/citations/19900004884/downloads/19900004884.pdf>, page 38
    pub fn induced_velocity_from_panel_as_point_doublet_with_unit_strength(
        &self, 
        area: f64,
        normal: SpatialVector<3>,
        center: SpatialVector<3>,
        ctrl_point: SpatialVector<3>
    ) -> SpatialVector<3> {
        let area_term = area / (4.0 * PI);

        let translated_point = ctrl_point - center;

        let distance = translated_point.length();
 
        let normal_height = translated_point.dot(normal);

        area_term * (3.0 * normal_height * translated_point - distance.powi(2) * normal) / distance.powi(5)
    }
}