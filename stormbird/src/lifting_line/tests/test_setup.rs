// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use crate::lifting_line::prelude::*;

#[derive(Debug, Clone)]
/// Struct for setting up line force models for a rectangular wing with constant foil properties
pub struct RectangularWing {
    pub aspect_ratio: f64,
    pub cl_zero_angle: f64,
    pub angle_of_attack: f64,
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
            -Vec3::unit_z()
        } else {
            Vec3::unit_z()
        };

        let chord_vector = Vec3::new(1.0, 0.0, 0.0).rotate_around_axis(
            -self.angle_of_attack, rotation_axis
        );

        let mut line_force_model_builder = LineForceModelBuilder::new(self.nr_strips);
        line_force_model_builder.density = 10.0;
        
        let last_z = if self.negative_span_orientation {
            -self.aspect_ratio
        } else {
            self.aspect_ratio
        };

        let wing_builder = WingBuilder {
            section_points: vec![
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 0.0, last_z),
            ],
            chord_vectors: vec![
                chord_vector,
                chord_vector,
            ],
            section_model: SectionModel::Foil(Foil {
                cl_zero_angle: self.cl_zero_angle,
                mean_positive_stall_angle: 45.0_f64.to_radians(),
                mean_negative_stall_angle: 45.0_f64.to_radians(),
                ..Default::default()
            }),
        };

        line_force_model_builder.add_wing(wing_builder);

        line_force_model_builder
    }
}