
//! Tests for the line force model functionality.


pub mod motion;

use crate::line_force_model::single_wing::WingBuilder;
use crate::line_force_model::builder::LineForceModelBuilder;
use crate::line_force_model::LineForceModel;

use crate::section_models::SectionModel;
use crate::section_models::foil::Foil;

use stormath::spatial_vector::SpatialVector;

/// Returns an example line force model with two wings, oriented along the z-axis.
pub fn get_example_model() -> LineForceModel {
    let chord_length = 11.0;
    let span = 33.0;
    let start_height = 5.2;

    let mut builder = LineForceModelBuilder::new(5);

    let chord_vector = SpatialVector([chord_length, 0.0, 0.0]);

    let x_positions = vec![-1.5 * span, 1.32 * span];

    for x in x_positions {
        let wing = WingBuilder{
            section_points: vec![
                SpatialVector([x, 0.0, start_height]),
                SpatialVector([x, 0.0, start_height + span]),
            ],
            chord_vectors: vec![
                chord_vector,
                chord_vector,
            ],
            section_model: SectionModel::Foil(Foil::default()),
            non_zero_circulation_at_ends: [false, false],
            nr_sections: None,
        };

        builder.add_wing(wing);
    }

    builder.build()
}
