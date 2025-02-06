// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Funtctionality for vortex lines and their induced velocities.
use std::f64::consts::PI;

use math_utils::spatial_vector::SpatialVector;

const CLOSENESS_ERROR: f64 = 1.0e-10;

#[inline(always)]
/// Implementation of induced velocity function based on the user manual for VSAERO
/// Link: <https://ntrs.nasa.gov/api/citations/19900004884/downloads/19900004884.pdf>
pub fn induced_velocity_from_line_with_unit_strength(
    line_points: &[SpatialVector<3>; 2], 
    ctrl_point: SpatialVector<3>, 
    viscous_core_length: f64,
) -> SpatialVector<3> {
    let r_1 = ctrl_point - line_points[0];
    let r_2 = ctrl_point - line_points[1];

    let r_1_length = r_1.length();
    let r_2_length = r_2.length(); 

    let r_1_r_2 = r_1_length * r_2_length;

    let denominator = r_1_r_2 * (r_1_r_2 + r_1.dot(r_2));

    if denominator.abs() > CLOSENESS_ERROR {
        let viscous_core_term = if viscous_core_length == 0.0 {
            1.0
        } else {
            viscous_core_term(line_points, ctrl_point, viscous_core_length)
        };

        let k = (r_1_length + r_2_length) / denominator;

        viscous_core_term * r_1.cross(r_2) * (k / (4.0 * PI))
    }
    else {
        SpatialVector::<3>::default()
    }
}

#[inline(always)]
/// Calculates the distance between the point and the line
pub fn normal_distance(line_points: &[SpatialVector<3>; 2], ctrl_point: SpatialVector<3>) -> f64 {
    let relative_line  = line_points[1] - line_points[0];
    let relative_point = ctrl_point - line_points[0];

    let line_direction = relative_line.normalize();

    let point_vector_line_parallel = relative_point.project(line_direction);

    let parallel_distance_from_start_point = point_vector_line_parallel.dot(line_direction);

    if parallel_distance_from_start_point < 0.0 {
        relative_point.length()
    } else if parallel_distance_from_start_point > relative_line.length() {
        (ctrl_point - line_points[1]).length()
    } else {
        let point_vector_line_normal  = relative_point - point_vector_line_parallel;

        point_vector_line_normal.length()
    }
}

#[inline(always)]
/// Viscous core term. Based on expressions from:
/// J. T. Reid (2020) - A general approach to lifting-line theory, applied to wings with sweep
/// Link: <https://digitalcommons.usu.edu/cgi/viewcontent.cgi?article=8982&context=etd>
pub fn viscous_core_term(line_points: &[SpatialVector<3>; 2], ctrl_point: SpatialVector<3>, viscous_core_length: f64) -> f64 {
    let distance = normal_distance(line_points, ctrl_point);

    let denominator = (viscous_core_length.powi(4) + distance.powi(4)).sqrt();
    
    if denominator > 0.0 {
        distance.powi(2) / denominator
    } else {
        0.0
    }
    
}