// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)


use stormath::spatial_vector::SpatialVector;
use stormath::spatial_vector::geometry_functions;
use stormath::type_aliases::Float;
use stormath::consts::PI;

const FOUR_PI_INVERSE: Float = 1.0/ (4.0 * PI);

use super::vortex_line;

#[derive(Clone, Debug, Default)]
pub struct Panel {
    points: [SpatialVector; 4],
    center: SpatialVector,
    normal: SpatialVector,
    far_field_length_squared: Float,
    viscous_core_length: Float,
    point_doublet_area_term: Float
}

impl Panel {
    pub fn new(
        points: [SpatialVector; 4],
        far_field_ratio: Float,
        viscous_core_length: Float,
    ) -> Self {
        let center = 0.25 * (points[0] + points[1] + points[2] + points[3]);
        let area = geometry_functions::area_of_quadrilateral(&points);
        let normal = geometry_functions::normal_of_quadrilateral(&points);

        let bound_vortex_length = (points[1] - points[0]).length();
        let stream_length = (points[2] - points[1]).length();

        let representative_length = bound_vortex_length.max(stream_length);

        let far_field_length_squared = (representative_length * far_field_ratio).powi(2);

        Self {
            points,
            center,
            normal,
            far_field_length_squared,
            viscous_core_length,
            point_doublet_area_term: area * FOUR_PI_INVERSE,
        }
    }

    #[inline(always)]
    pub fn necessary_with_full_vortex_line_computation(&self, ctrl_point: SpatialVector) -> bool {
        let distance_to_ctrl_point_sq = (ctrl_point - self.center).length_squared();

        distance_to_ctrl_point_sq <= self.far_field_length_squared
    }

    /// Function that calculates the induced velocity from a vortex panel, based on the corner points
    /// given as input
    pub fn induced_velocity_with_unit_strength(
        &self, 
        ctrl_point: SpatialVector,
    ) -> SpatialVector {
        let full_computation_needed = self.necessary_with_full_vortex_line_computation(ctrl_point);

        if full_computation_needed {
            self.induced_velocity_as_vortex_lines_with_unit_strength(
                ctrl_point,
            )
        } else {
            self.induced_velocity_as_point_doublet_with_unit_strength(
                ctrl_point, 
            )
        }
    }

    #[inline(always)]
    /// Simplified formulation for a panel, based on a point formulation.
    /// Based on the equations from: 
    /// <https://ntrs.nasa.gov/api/citations/19900004884/downloads/19900004884.pdf>, page 38
    pub fn induced_velocity_as_point_doublet_with_unit_strength(
        &self, 
        ctrl_point: SpatialVector
    ) -> SpatialVector {
        let translated_point = ctrl_point - self.center;

        let distance_squared = translated_point.length_squared();
        let distance_pow_5 = distance_squared * distance_squared * distance_squared.sqrt();
 
        let normal_height = translated_point.dot(self.normal);

        self.point_doublet_area_term * 
        (3.0 * normal_height * translated_point - distance_squared * self.normal) / 
        distance_pow_5
    }

    #[inline(always)]
    pub fn induced_velocity_as_vortex_lines_with_unit_strength(
        &self, 
        ctrl_point: SpatialVector,
    ) -> SpatialVector {
        let mut u_i: SpatialVector = SpatialVector::default();

        for i_point in 0..self.points.len() {
            let line_points: [SpatialVector; 2] = if i_point == self.points.len() - 1 {
                [self.points[i_point], self.points[0]]
            } else {
                [self.points[i_point], self.points[i_point + 1]]
            };

            u_i += vortex_line::induced_velocity_from_line_with_unit_strength(
                &line_points, ctrl_point, self.viscous_core_length
            );
        }

        u_i
    }
}