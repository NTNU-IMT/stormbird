// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Funtctionality for vortex lines and their induced velocities.
use std::f64::consts::PI;

use math_utils::spatial_vector::SpatialVector;
use super::PotentialTheoryModel;
use super::viscous_core_length::ViscousCoreLength;

impl PotentialTheoryModel {
    /// Implementation of induced velocity function based on the user manual for VSAERO
    /// Link: <https://ntrs.nasa.gov/api/citations/19900004884/downloads/19900004884.pdf>
    pub fn induced_velocity_from_line_with_unit_strength(
        line_points: &[SpatialVector<3>; 2], 
        ctrl_point: SpatialVector<3>, 
        viscous_core_length: ViscousCoreLength,
        closeness_error: f64,
    ) -> SpatialVector<3> {
        let r_1 = ctrl_point - line_points[0];
        let r_2 = ctrl_point - line_points[1];

        let r_1_length = r_1.length();
        let r_2_length = r_2.length(); 

        let r_1_r_2 = r_1_length * r_2_length;

        let denominator = r_1_r_2 * (r_1_r_2 + r_1.dot(r_2));

        if denominator.abs() > closeness_error {
            let viscous_core_term = if let ViscousCoreLength::NoViscousCore = viscous_core_length {
                1.0
            } else {
                viscous_core_length.viscous_core_term(line_points, ctrl_point)
            };

            let k = (r_1_length + r_2_length) / denominator;

            viscous_core_term * r_1.cross(r_2) * (k / (4.0 * PI))
        }
        else {
            SpatialVector::<3>::default()
        }
    }
}