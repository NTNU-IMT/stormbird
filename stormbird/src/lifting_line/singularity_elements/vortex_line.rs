// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Funtctionality for vortex lines and their induced velocities.

use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;
use stormath::consts::PI;

const FOUR_PI_INVERSE: Float = 1.0 / (4.0 * PI);

const CLOSENESS_ERROR: Float = 1.0e-10;

#[inline(always)]
/// Implementation of induced velocity function based on the user manual for VSAERO
/// Link: <https://ntrs.nasa.gov/api/citations/19900004884/downloads/19900004884.pdf>
pub fn induced_velocity_from_line_with_unit_strength(
    line_points: &[SpatialVector; 2], 
    ctrl_point: SpatialVector, 
    viscous_core_length: Float,
) -> SpatialVector {
    let r_1 = ctrl_point - line_points[0];
    let r_2 = ctrl_point - line_points[1];

    let r_1_length_sq = r_1.length_squared();
    let r_2_length_sq = r_2.length_squared();
    let r_1_length = r_1_length_sq.sqrt();
    let r_2_length = r_2_length_sq.sqrt();

    let r_1_r_2 = r_1_length * r_2_length;

    let denominator = r_1_r_2 * (r_1_r_2 + r_1.dot(r_2));

    if denominator.abs() > CLOSENESS_ERROR {
        let viscous_core_term = if viscous_core_length == 0.0 {
            1.0
        } else {
            viscous_core_term(
                line_points, 
                ctrl_point, 
                viscous_core_length,
                r_1_length_sq,
                r_2_length_sq
            )
        };

        let k = (r_1_length + r_2_length) / denominator;

        viscous_core_term * r_1.cross(r_2) * (k * FOUR_PI_INVERSE)
    }
    else {
        SpatialVector::default()
    }
}

#[inline(always)]
/// Calculates the distance between the point and the line
fn normal_distance_squared(
    line_points: &[SpatialVector; 2], 
    ctrl_point: SpatialVector,
    r_1_length_sq: Float,
    r_2_length_sq: Float,
) -> Float {
    let relative_line  = line_points[1] - line_points[0];
    let relative_point = ctrl_point - line_points[0];

    let line_length_sq = relative_line.length_squared();
    let line_length = line_length_sq.sqrt();
    let line_direction = relative_line / line_length;

    //let line_direction = relative_line.normalize();

    let parallel_distance_from_start = relative_point.dot(line_direction);

    if parallel_distance_from_start < 0.0 {
        r_1_length_sq
    } else if parallel_distance_from_start > line_length {
        r_2_length_sq
    } else {
        let parallel_component_sq = parallel_distance_from_start * parallel_distance_from_start;
        relative_point.length_squared() - parallel_component_sq
    }
}

#[inline(always)]
/// Viscous core term. Based on expressions from:
/// J. T. Reid (2020) - A general approach to lifting-line theory, applied to wings with sweep
/// Link: <https://digitalcommons.usu.edu/cgi/viewcontent.cgi?article=8982&context=etd>
fn viscous_core_term(
    line_points: &[SpatialVector; 2], 
    ctrl_point: SpatialVector, 
    viscous_core_length: Float,
    r_1_length_sq: Float,
    r_2_length_sq: Float,
) -> Float {
    let distance_squared = normal_distance_squared(line_points, ctrl_point, r_1_length_sq, r_2_length_sq);

    let denominator = (viscous_core_length.powi(4) + distance_squared * distance_squared).sqrt();
    
    if denominator > 0.0 {
        distance_squared / denominator
    } else {
        0.0
    }
}