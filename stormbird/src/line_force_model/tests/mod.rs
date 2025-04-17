

use crate::line_force_model::single_wing::WingBuilder;
use crate::line_force_model::builder::LineForceModelBuilder;

use crate::section_models::SectionModel;
use crate::section_models::foil::Foil;

use math_utils::spatial_vector::SpatialVector;

#[test]
fn test_wing_angles() {
    let mut builder = LineForceModelBuilder::new(5);

    let chord_vector = SpatialVector([1.0, 0.0, 0.0]);

    let x_positions = vec![-1.5, 1.32];

    let rotation_angle = 45.0_f64.to_radians();

    let rotated_chord_vector = chord_vector.rotate_around_axis(rotation_angle, SpatialVector([0.0, 0.0, 1.0]));

    for x in x_positions {
        let wing = WingBuilder{
            section_points: vec![
                SpatialVector([x, 0.0, 0.0]),
                SpatialVector([x, 0.0, 1.0]),
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

    let mut line_force_model = builder.build();

    let original_span_points = line_force_model.span_points();

    let wing_angles = vec![rotation_angle, rotation_angle];

    line_force_model.local_wing_angles = wing_angles.clone();

    let chord_vectors = line_force_model.global_chord_vectors();
    let span_points = line_force_model.span_points();

    for i in 0..chord_vectors.len() {
        assert_eq!(chord_vectors[i], rotated_chord_vector);
    }

    for i in 0..span_points.len() {
        assert_eq!(span_points[i], original_span_points[i]);
    }
}