// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use serde::{Serialize, Deserialize};

use crate::vec3::Vec3;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Enum to represent different ways a viscous core length can be specified. 
pub enum ViscousCoreLength {
    /// Signifies that the viscous core length is a fraction of the length of the vortex line. To 
    /// be used, the vortex line geometry must be known.
    Relative(f64),
    /// Signifies that the viscous core length is an absolute value, and that it can be used without
    /// any more information about the geometry.
    Absolute(f64),
}

impl Default for ViscousCoreLength {
    fn default() -> Self {
        Self::Relative(0.1)
    }
}

impl ViscousCoreLength {
    /// Calculates the distance between the point and the line
    pub fn distance(&self, line_points: &[Vec3; 2], ctrl_point: Vec3) -> f64 {
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
    /// Returns the viscous core length as an absolute value
    pub fn absolute_viscous_core_length(&self, line_length: f64) -> f64 {
        match self {
            Self::Relative(relative_length) => relative_length * line_length,
            Self::Absolute(absolute_length) => *absolute_length,
        }
    }

    /// Viscous core term. Based on expressions from:
    /// J. T. Reid (2020) - A general approach to lifting-line theory, applied to wings with sweep
    /// Link: <https://digitalcommons.usu.edu/cgi/viewcontent.cgi?article=8982&context=etd>
    pub fn viscous_core_term(&self, line_points: &[Vec3; 2], ctrl_point: Vec3) -> f64 {
        let distance = self.distance(line_points, ctrl_point);

        let line_length = (line_points[1] - line_points[0]).length();

        let viscous_core_length = self.absolute_viscous_core_length(line_length);

        let denominator = (viscous_core_length.powi(4) + distance.powi(4)).sqrt();
        
        if denominator > 0.0 {
            distance.powi(2) / denominator
        } else {
            0.0
        }
        
    }
}


