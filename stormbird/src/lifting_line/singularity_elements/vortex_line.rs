// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Functionality for vortex lines and their induced velocities.

use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;
use stormath::consts::PI;

const FOUR_PI_INVERSE: Float = 1.0 / (4.0 * PI);

#[cfg(feature = "single_precision")]
const CLOSENESS_ERROR: Float = 1.0e-6;

#[cfg(not(feature = "single_precision"))]
const CLOSENESS_ERROR: Float = 1.0e-10;

#[derive(Clone, Debug, Default)]
pub struct VortexLine {
    pub points: [SpatialVector; 2],
    pub length: Float,
    pub length_squared: Float,
    pub direction: SpatialVector,
}

impl VortexLine {
    pub fn new(points: [SpatialVector; 2]) -> Self {
        let relative_line  = points[1] - points[0];
    
        let length_squared = relative_line.length_squared();
        let length = length_squared.sqrt().max(CLOSENESS_ERROR);

        let direction = relative_line / length;

        Self {
            points,
            length,
            length_squared,
            direction
        }
    }
}

impl VortexLine {
    #[inline(always)]
    /// Implementation of induced velocity function based on the user manual for VSAERO
    /// Link: <https://ntrs.nasa.gov/api/citations/19900004884/downloads/19900004884.pdf>
    pub fn induced_velocity_from_line_with_unit_strength(
        &self,
        ctrl_point: SpatialVector, 
        viscous_core_length: Float,
    ) -> SpatialVector {
        let r_1 = ctrl_point - self.points[0];
        let r_2 = ctrl_point - self.points[1];
    
        let r_1_length_sq = r_1.length_squared();
        let r_1_length = r_1_length_sq.sqrt();
        let r_2_length = r_2.length();
    
        let r_1_r_2 = r_1_length * r_2_length;

        let denominator = (r_1_r_2 * (r_1_r_2 + r_1.dot(r_2))).max(CLOSENESS_ERROR);
    
        let viscous_core_term = self.viscous_core_term(
            ctrl_point, 
            viscous_core_length,
            r_1_length_sq
        );

        let k = (r_1_length + r_2_length) / denominator;

        viscous_core_term * r_1.cross(r_2) * (k * FOUR_PI_INVERSE)
    }
    
    #[inline(always)]
    /// Calculates the distance between the point and the line
    fn normal_distance_squared(
        &self,
        ctrl_point: SpatialVector,
        r_1_length_sq: Float
    ) -> Float {
        let relative_point = ctrl_point - self.points[0];

        let t = relative_point.dot(self.direction);
        let t_clamped = t.clamp(0.0, self.length);
        
        // distance_sq = |relative_point|^2 + t_clamped * (t_clamped - 2*t)
        t_clamped.mul_add(t_clamped - 2.0 * t, r_1_length_sq)
    }
    
    #[inline(always)]
    /// Viscous core term. Based on expressions from:
    /// J. T. Reid (2020) - A general approach to lifting-line theory, applied to wings with sweep
    /// Link: <https://digitalcommons.usu.edu/cgi/viewcontent.cgi?article=8982&context=etd>
    fn viscous_core_term(
        &self,
        ctrl_point: SpatialVector, 
        viscous_core_length: Float,
        r_1_length_sq: Float
    ) -> Float {
        let distance_squared = self.normal_distance_squared(ctrl_point, r_1_length_sq);
    
        let denominator = distance_squared.mul_add(
            distance_squared, 
            viscous_core_length.powi(4)
        ).sqrt().max(CLOSENESS_ERROR);
        
        distance_squared / denominator
    }
}
