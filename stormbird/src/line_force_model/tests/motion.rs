
use super::*;

use stormath::type_aliases::Float;
use stormath::consts::PI;


#[test]
/// Tests the rigid body motion of the line force model by comparing the computed velocity at the 
/// control points with an estimate based on finite difference.
fn test_rigid_body_motion() {
    let mut line_force_model = get_example_model();

    let rotation_motion_period = 1.0;
    let translation_motion_period = 2.0;

    // Sway motion
    let translation_func = |time: Float| -> SpatialVector {
        let frequency = 1.0 / translation_motion_period;
        let amplitude = 1.0;
        let omega = 2.0 * PI * frequency;

        SpatialVector::from([0.0, amplitude * (omega * time).sin(), 0.0])
    };

    // Roll motion
    let rotation_func = |time: Float| -> SpatialVector {
        let frequency = 1.0 / rotation_motion_period;
        let amplitude = Float::from(45.0).to_radians();
        let omega = 2.0 * PI * frequency;

        SpatialVector::from([amplitude * (omega * time).sin(), 0.0, 0.0])
    };

    let mut time = 0.0;
    let time_step = rotation_motion_period / 1000.0;
    let end_time = 100.0 * time_step;

    let acceptable_error = 0.01; // The difference in the calculated velocity magnitudes should be less than this value.

    let mut previous_ctrl_points = line_force_model.ctrl_points();

    while time < end_time {
        line_force_model.rigid_body_motion.update_translation_with_velocity_using_finite_difference(
            translation_func(time), 
            time_step
        );

        line_force_model.rigid_body_motion.update_rotation_with_velocity_using_finite_difference(
            rotation_func(time), 
            time_step
        );

        let ctrl_points = line_force_model.ctrl_points();

        let ctrl_points_velocity_fd: Vec<SpatialVector> = ctrl_points.iter()
            .zip(previous_ctrl_points.iter())
            .map(|(current, previous)| (*current - *previous) / time_step)
            .collect();

        let ctrl_points_velocity_rb = line_force_model.rigid_body_motion.velocities_at_points(&ctrl_points);

        for i in 0..ctrl_points.len() {
            let velocity_mag_fd = ctrl_points_velocity_fd[i].length();

            if velocity_mag_fd <= 1e-6 {
                continue;
            }

            let difference = (ctrl_points_velocity_fd[i] - ctrl_points_velocity_rb[i]).length() / velocity_mag_fd;

            assert!(difference < acceptable_error, "Length of difference between finite difference and rigid body: {}", difference)
        }

        previous_ctrl_points = ctrl_points.clone();

        time += time_step;
    }

}

#[test]
/// Tests that the applying the wing angles to the line force model rotates the chord vector in the
/// right way *and* that the span points are not affected by the rotation.
fn test_wing_angles() {
    let mut line_force_model = get_example_model();

    let chord_vector = line_force_model.chord_vectors_local[0].clone();

    let rotation_angle = Float::from(45.0).to_radians();

    let rotated_chord_vector = chord_vector.rotate_around_axis(
        rotation_angle, 
        SpatialVector::from([0.0, 0.0, 1.0])
    );

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