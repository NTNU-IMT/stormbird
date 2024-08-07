// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use std::f64::consts::PI;

use math_utils::spatial_vector::SpatialVector;
use math_utils::spatial_vector::geometry_functions;

use super::PotentialTheoryModel;
use super::viscous_core_length::ViscousCoreLength;


#[derive(Clone, Debug)]
/// Data necessary when calculating induced velocities from a quadrilateral panel in a simplified 
/// manner.
pub struct PanelGeometry {
    pub area: f64,
    pub normal: SpatialVector<3>,
    pub center: SpatialVector<3>,
}

impl Default for PanelGeometry {
    /// Default implementation used when initializing a vector with panels
    fn default() -> Self {
        Self {
            area: 0.0,
            normal: SpatialVector::<3>::default(),
            center: SpatialVector::<3>::default(),
        }
    }
}

impl PanelGeometry {
    pub fn new(points: [SpatialVector<3>; 4]) -> Self {
        let area   = geometry_functions::area_of_quadrilateral(&points);
        let normal = geometry_functions::normal_of_quadrilateral(&points);
        
        let center = 0.25 * (points[0] + points[1] + points[2] + points[3]);
        
        Self {
            area,
            normal,
            center,
        }
    }

    pub fn representative_length(&self) -> f64 {
        self.area.sqrt()
    }
}

impl PotentialTheoryModel {
    /// Function that calculates the induced velocity from a vortex panel, based on the corner points
    /// given as input
    pub fn induced_velocity_from_panel_with_unit_strength(
        &self, 
        panel_points: &[SpatialVector<3>; 4], 
        panel: &PanelGeometry, 
        ctrl_point: SpatialVector<3>,
        off_body: bool,
    ) -> SpatialVector<3> {
        let distance_to_ctrl_point = (ctrl_point - panel.center).length();

        let u_i = if distance_to_ctrl_point > panel.representative_length() * self.far_field_ratio {
            self.induced_velocity_from_panel_as_point_doublet_with_unit_strength(panel, ctrl_point)
        } else {
            self.induced_velocity_from_panel_as_vortex_lines_with_unit_strength(panel_points, ctrl_point, off_body)
        };

        let ctrl_point_mirrored = self.symmetry_condition.mirrored_point(ctrl_point);

        if let Some(ctrl_point_mirrored) = ctrl_point_mirrored {
            let mirrored_distance_to_ctrl_point = (ctrl_point_mirrored - panel.center).length();

            let u_i_m = if mirrored_distance_to_ctrl_point > panel.representative_length() * self.far_field_ratio {
                self.induced_velocity_from_panel_as_point_doublet_with_unit_strength(panel, ctrl_point_mirrored)
            } else {
                self.induced_velocity_from_panel_as_vortex_lines_with_unit_strength(panel_points, ctrl_point_mirrored, off_body)
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
        off_body: bool,
    ) -> SpatialVector<3> {
        let mut u_i: SpatialVector<3> = SpatialVector::<3>::default();

        let viscous_core_length_raw = if off_body && self.viscous_core_length_off_body.is_some() {
            self.viscous_core_length_off_body.unwrap()
        } else {
            self.viscous_core_length
        };

        // Use the bound vortex length as the representative length for all lines in the panel
        let viscous_core_length = match viscous_core_length_raw {
            ViscousCoreLength::Relative(core_length) => {
                let representative_length = (panel_points[1] - panel_points[0]).length();

                ViscousCoreLength::Absolute(core_length * representative_length)
            },
            ViscousCoreLength::Absolute(core_length) => ViscousCoreLength::Absolute(core_length),
            ViscousCoreLength::NoViscousCore => ViscousCoreLength::NoViscousCore,
        };

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

    /// Simplfied formulation for a panel, based on a point formulation.
    /// Based on the equations from: 
    /// <https://ntrs.nasa.gov/api/citations/19900004884/downloads/19900004884.pdf>, page 38
    pub fn induced_velocity_from_panel_as_point_doublet_with_unit_strength(
        &self, 
        panel: &PanelGeometry, 
        ctrl_point: SpatialVector<3>
    ) -> SpatialVector<3> {
        let area_term = panel.area / (4.0 * PI);

        let translated_point = ctrl_point - panel.center;

        let distance = translated_point.length();
 
        let normal_height = translated_point.dot(panel.normal);

        area_term * (3.0 * normal_height * translated_point - distance.powi(2) * panel.normal) / distance.powi(5)
    }
}