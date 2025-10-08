// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use crate::lifting_line::prelude::*;

use stormath::type_aliases::Float;


#[derive(Debug, Clone)]
/// Struct for setting up line force models for a rectangular wing with constant foil properties
pub struct RectangularWing {
    pub aspect_ratio: Float,
    pub cl_zero_angle: Float,
    pub angle_of_attack: Float,
    pub nr_strips: usize,
    pub negative_span_orientation: bool
}

impl Default for RectangularWing {
    fn default() -> Self {
        Self {
            aspect_ratio: 5.0,
            cl_zero_angle: 0.5,
            angle_of_attack: 0.0,
            nr_strips: 20,
            negative_span_orientation: false
        }
    }
}

impl RectangularWing {
    pub fn build(&self) -> LineForceModelBuilder {
        let rotation_axis = if self.negative_span_orientation {
            -SpatialVector::unit_z()
        } else {
            SpatialVector::unit_z()
        };

        let chord_vector = SpatialVector::from([1.0, 0.0, 0.0]).rotate_around_axis(
            -self.angle_of_attack, rotation_axis
        );

        let mut line_force_model_builder = LineForceModelBuilder::new(self.nr_strips);
        line_force_model_builder.density = 13.2; // Sets this to a large 'random' value to detect any errors due to incorrect density

        
        let last_z = if self.negative_span_orientation {
            -self.aspect_ratio
        } else {
            self.aspect_ratio
        };

        let wing_builder = WingBuilder {
            section_points: vec![
                SpatialVector::from([0.0, 0.0, 0.0]),
                SpatialVector::from([0.0, 0.0, last_z]),
            ],
            chord_vectors: vec![
                chord_vector,
                chord_vector,
            ],
            section_model: SectionModel::Foil(Foil {
                cl_zero_angle: self.cl_zero_angle,
                mean_positive_stall_angle: Float::from(45.0).to_radians(),
                mean_negative_stall_angle: Float::from(45.0).to_radians(),
                ..Default::default()
            }),
            non_zero_circulation_at_ends: [false, false],
            ..Default::default()
        };

        line_force_model_builder.add_wing(wing_builder);

        line_force_model_builder
    }
}