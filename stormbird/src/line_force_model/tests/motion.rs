// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use super::*;

use stormath::type_aliases::Float;
use stormath::consts::PI;
use stormath::spatial_vector::transformations::RotationType;

use crate::common_utils::forces_and_moments::CoordinateSystem;
use crate::common_utils::results::solver::SolverResult;
use crate::test_utils::tolerance;


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
        let amplitude = Float::to_radians(45.0);
        let omega = 2.0 * PI * frequency;

        SpatialVector::from([amplitude * (omega * time).sin(), 0.0, 0.0])
    };

    let mut time = 0.0;
    let time_step = rotation_motion_period / 1000.0;
    let end_time = 100.0 * time_step;

    let acceptable_error = 0.01; // The difference in the calculated velocity magnitudes should be less than this value.

    let mut previous_ctrl_points = line_force_model.ctrl_points_global.clone();

    while time < end_time {
        line_force_model.set_translation_and_rotation_with_finite_difference_for_the_velocity(
            time_step, translation_func(time), rotation_func(time)
        );

        let ctrl_points = line_force_model.ctrl_points_global.clone();

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

/// Harmonic motion in all six degrees of freedom, around a mean heading, with analytical
/// expressions for the derivatives
struct SixDofMotion {
    translation_amplitude: [Float; 3],
    rotation_mean: [Float; 3],
    rotation_amplitude: [Float; 3],
    angular_frequency: [Float; 6],
    phase: [Float; 6],
}

impl SixDofMotion {
    fn example() -> Self {
        Self {
            translation_amplitude: [1.5, 2.0, 0.8],
            rotation_mean: [0.0, 0.0, Float::to_radians(30.0)],
            rotation_amplitude: [
                Float::to_radians(15.0),
                Float::to_radians(5.0),
                Float::to_radians(8.0),
            ],
            angular_frequency: [
                2.0 * PI / 7.0, 2.0 * PI / 9.0, 2.0 * PI / 5.0,
                2.0 * PI / 8.0, 2.0 * PI / 6.0, 2.0 * PI / 11.0,
            ],
            phase: [0.0, 0.3, 1.1, 0.5, 2.0, -0.7],
        }
    }

    fn translation(&self, time: Float) -> SpatialVector {
        SpatialVector::from(std::array::from_fn(|i| {
            self.translation_amplitude[i] * (self.angular_frequency[i] * time + self.phase[i]).sin()
        }))
    }

    fn translation_derivative(&self, time: Float) -> SpatialVector {
        SpatialVector::from(std::array::from_fn(|i| {
            self.translation_amplitude[i] * self.angular_frequency[i] *
                (self.angular_frequency[i] * time + self.phase[i]).cos()
        }))
    }

    fn rotation(&self, time: Float) -> SpatialVector {
        SpatialVector::from(std::array::from_fn(|i| {
            self.rotation_mean[i] + self.rotation_amplitude[i] *
                (self.angular_frequency[i + 3] * time + self.phase[i + 3]).sin()
        }))
    }

    /// The time derivative of the rotation *angles*, which is not the angular velocity
    fn rotation_derivative(&self, time: Float) -> SpatialVector {
        SpatialVector::from(std::array::from_fn(|i| {
            self.rotation_amplitude[i] * self.angular_frequency[i + 3] *
                (self.angular_frequency[i + 3] * time + self.phase[i + 3]).cos()
        }))
    }
}

/// Angular velocity in the global coordinate system, from the angles and the time derivative of
/// the angles. The rate of each rotation acts around its axis, rotated by the rotations that are
/// applied *after* it. For instance, ω = φ̇ Rz Ry ex + θ̇ Rz ey + ψ̇ ez for `XYZ`.
fn analytical_angular_velocity_global(
    angles: SpatialVector,
    angle_rates: SpatialVector,
    rotation_type: RotationType
) -> SpatialVector {
    let order = rotation_type.axis_order();
    let unit_vectors = [SpatialVector::unit_x(), SpatialVector::unit_y(), SpatialVector::unit_z()];

    let mut angular_velocity = SpatialVector::default();

    for i in 0..3 {
        let mut contribution = unit_vectors[order[i]] * angle_rates[order[i]];

        for &axis in &order[i + 1..] {
            contribution = contribution.rotate_around_coordinate_axis(axis, angles[axis]);
        }

        angular_velocity += contribution;
    }

    angular_velocity
}

/// Angular velocity in the body fixed coordinate system, e.g., (p, q, r) for a ship. The rate of
/// each rotation acts around its axis, rotated back by the inverse of the rotations that are
/// applied *before* it. For instance, ω_body = φ̇ ex + θ̇ Rxᵀ ey + ψ̇ Rxᵀ Ryᵀ ez for `XYZ`.
fn analytical_angular_velocity_body_fixed(
    angles: SpatialVector,
    angle_rates: SpatialVector,
    rotation_type: RotationType
) -> SpatialVector {
    let order = rotation_type.axis_order();
    let unit_vectors = [SpatialVector::unit_x(), SpatialVector::unit_y(), SpatialVector::unit_z()];

    let mut angular_velocity = SpatialVector::default();

    for i in 0..3 {
        let mut contribution = unit_vectors[order[i]] * angle_rates[order[i]];

        for &axis in order[..i].iter().rev() {
            contribution = contribution.rotate_around_coordinate_axis(axis, -angles[axis]);
        }

        angular_velocity += contribution;
    }

    angular_velocity
}

#[test]
/// Checks the analytical body fixed angular velocity against the textbook expressions for the roll,
/// pitch and yaw rates of a ship, for the `XYZ` rotation type (yaw, then pitch, then roll around
/// the rotated axes). Anchors the helper functions used in the other motion tests.
fn analytical_angular_velocity_matches_ship_kinematics() {
    let angles = SpatialVector::new(0.3, -0.2, 2.1);
    let rates = SpatialVector::new(0.05, -0.11, 0.07);

    let (phi, theta) = (angles[0], angles[1]);

    let expected_body = SpatialVector::new(
        rates[0] - rates[2] * theta.sin(),
        rates[1] * phi.cos() + rates[2] * theta.cos() * phi.sin(),
        -rates[1] * phi.sin() + rates[2] * theta.cos() * phi.cos(),
    );

    let body = analytical_angular_velocity_body_fixed(angles, rates, RotationType::XYZ);
    let global = analytical_angular_velocity_global(angles, rates, RotationType::XYZ);

    assert!((body - expected_body).length() < tolerance(1e-12, 1e-6), "body: {:?}, expected {:?}", body, expected_body);

    // The global vector is the body fixed vector rotated by the full rotation
    let global_from_body = expected_body.rotate(angles, RotationType::XYZ);

    assert!((global - global_from_body).length() < tolerance(1e-12, 1e-6), "global: {:?}, from body: {:?}", global, global_from_body);
}

/// The ways the velocity of the rigid body motion can be defined
#[derive(Debug, Clone, Copy)]
enum VelocityDefinition {
    /// Computed by the rigid body motion from the change in translation and rotation
    FiniteDifference,
    /// Set directly from the analytical velocities in the global coordinate system
    AnalyticalGlobal,
    /// Analytical velocities in the body fixed coordinate system, e.g., (u, v, w) and (p, q, r)
    /// from a ship simulator, converted to the global coordinate system as in the FMU
    AnalyticalBodyFixed,
}

#[test]
/// Tests the rigid body motion of the line force model for a combined motion in all six degrees of
/// freedom, for all rotation types, with the output in both the global and the body fixed
/// coordinate system, and with the velocity of the motion defined in three different ways (see
/// [`VelocityDefinition`]).
///
/// The reference is a central finite difference of the position of the control points. The test
/// checks:
/// - the motion velocity at the control points
/// - the felt velocity and the angular velocity in the sectional force input, which are given in
///   the output coordinate system
/// - the felt velocity minus the rotational motion in the simulation result, which should be the
///   negative of the linear velocity of the body, in the global coordinate system
fn rigid_body_motion_in_six_degrees_of_freedom() {
    let motion = SixDofMotion::example();

    // Half the step between the positions in the central difference. The finite difference
    // definition of the velocity uses the same positions, i.e., a step of 2 * dt.
    let dt = tolerance(1e-3, 2e-2);
    let allowed_relative_error = tolerance(1e-5, 2e-3);

    let rotation_types = [
        RotationType::XYZ, RotationType::XZY, RotationType::YXZ,
        RotationType::YZX, RotationType::ZXY, RotationType::ZYX,
    ];

    let coordinate_systems = [CoordinateSystem::Global, CoordinateSystem::Body];

    let velocity_definitions = [
        VelocityDefinition::FiniteDifference,
        VelocityDefinition::AnalyticalGlobal,
        VelocityDefinition::AnalyticalBodyFixed,
    ];

    let times: Vec<Float> = (0..12).map(|i| 0.93 * i as Float).collect();

    for rotation_type in rotation_types {
        for coordinate_system in coordinate_systems {
            let mut builder = get_example_builder();
            builder.rotation_type = rotation_type;
            builder.output_coordinate_system = coordinate_system;

            let mut model = builder.build().unwrap();

            let nr_span_lines = model.nr_span_lines();

            for velocity_definition in velocity_definitions {
                for &time in &times {
                    let case = format!(
                        "{:?}, {:?} output, {:?}, t = {}",
                        rotation_type, coordinate_system, velocity_definition, time
                    );

                    // Reference velocity from the positions of the control points
                    model.set_translation_and_rotation(
                        motion.translation(time - dt), motion.rotation(time - dt)
                    );
                    let ctrl_points_before = model.ctrl_points_global.clone();

                    model.set_translation_and_rotation(
                        motion.translation(time + dt), motion.rotation(time + dt)
                    );
                    let ctrl_points_after = model.ctrl_points_global.clone();

                    let reference_velocity: Vec<SpatialVector> = (0..nr_span_lines).map(|i| {
                        (ctrl_points_after[i] - ctrl_points_before[i]) / (2.0 * dt)
                    }).collect();

                    let velocity_scale = reference_velocity.iter()
                        .map(|v| v.length())
                        .fold(0.0, Float::max);

                    let allowed_error = allowed_relative_error * velocity_scale;

                    // Analytical velocities
                    let rotation = motion.rotation(time);
                    let rotation_rates = motion.rotation_derivative(time);
                    let velocity_linear = motion.translation_derivative(time);

                    let angular_velocity_global = analytical_angular_velocity_global(
                        rotation, rotation_rates, rotation_type
                    );
                    let angular_velocity_body = analytical_angular_velocity_body_fixed(
                        rotation, rotation_rates, rotation_type
                    );

                    // Set the motion at the given time, with the velocity defined as in the case
                    match velocity_definition {
                        VelocityDefinition::FiniteDifference => {
                            model.set_translation_and_rotation(
                                motion.translation(time - dt), motion.rotation(time - dt)
                            );

                            model.set_translation_and_rotation_with_finite_difference_for_the_velocity(
                                2.0 * dt, motion.translation(time + dt), motion.rotation(time + dt)
                            );

                            // Move back to the center of the interval, keeping the velocity
                            model.set_translation_and_rotation(motion.translation(time), rotation);
                        },
                        VelocityDefinition::AnalyticalGlobal => {
                            model.set_translation_and_rotation(motion.translation(time), rotation);

                            model.rigid_body_motion.velocity_linear = velocity_linear;
                            model.rigid_body_motion.velocity_angular = angular_velocity_global;
                        },
                        VelocityDefinition::AnalyticalBodyFixed => {
                            model.set_translation_and_rotation(motion.translation(time), rotation);

                            let velocity_linear_body = model.rigid_body_motion
                                .vector_in_body_fixed_coordinate_system(velocity_linear);

                            model.rigid_body_motion.velocity_linear = model.rigid_body_motion
                                .vector_from_body_fixed_to_global_coordinate_system(velocity_linear_body);
                            model.rigid_body_motion.velocity_angular = model.rigid_body_motion
                                .vector_from_body_fixed_to_global_coordinate_system(angular_velocity_body);
                        },
                    }

                    // The motion velocity at the control points
                    let ctrl_points = model.ctrl_points_global.clone();
                    let motion_velocity = model.rigid_body_motion.velocities_at_points(&ctrl_points);

                    for i in 0..nr_span_lines {
                        let error = (motion_velocity[i] - reference_velocity[i]).length();

                        assert!(
                            error < allowed_error,
                            "{}: motion velocity at control point {}: {:?}, reference {:?}",
                            case, i, motion_velocity[i], reference_velocity[i]
                        );
                    }

                    // The results, with no wind, so that the felt velocity is due to the motion
                    let felt_velocity = model.felt_ctrl_points_velocity(
                        &vec![SpatialVector::default(); nr_span_lines]
                    );

                    let solver_result = SolverResult {
                        input_ctrl_points_velocity: felt_velocity.clone(),
                        circulation_strength: vec![0.0; nr_span_lines],
                        output_ctrl_points_velocity: felt_velocity,
                        iterations: 0,
                        residual: 0.0,
                    };

                    let result = model.calculate_simulation_result(
                        &solver_result,
                        &vec![SpatialVector::default(); nr_span_lines],
                        time
                    );

                    let expected_angular_velocity = match coordinate_system {
                        CoordinateSystem::Global => angular_velocity_global,
                        CoordinateSystem::Body => angular_velocity_body,
                    };

                    let angular_velocity_error = (
                        result.force_input.rotation_velocity - expected_angular_velocity
                    ).length();

                    assert!(
                        angular_velocity_error < allowed_relative_error * expected_angular_velocity.length(),
                        "{}: angular velocity in the force input: {:?}, expected {:?}",
                        case, result.force_input.rotation_velocity, expected_angular_velocity
                    );

                    let felt_velocity_minus_rotation = result.felt_velocity_minus_rotational_motion();

                    for i in 0..nr_span_lines {
                        let expected_felt_velocity_global = -reference_velocity[i];

                        let expected_felt_velocity = match coordinate_system {
                            CoordinateSystem::Global => expected_felt_velocity_global,
                            CoordinateSystem::Body => model.rigid_body_motion
                                .vector_in_body_fixed_coordinate_system(expected_felt_velocity_global),
                        };

                        let error = (result.force_input.velocity[i] - expected_felt_velocity).length();

                        assert!(
                            error < allowed_error,
                            "{}: felt velocity in the force input at control point {}: {:?}, expected {:?}",
                            case, i, result.force_input.velocity[i], expected_felt_velocity
                        );

                        let error = (felt_velocity_minus_rotation[i] + velocity_linear).length();

                        assert!(
                            error < allowed_error,
                            "{}: felt velocity minus rotational motion at control point {}: {:?}, expected {:?}",
                            case, i, felt_velocity_minus_rotation[i], -velocity_linear
                        );
                    }
                }
            }
        }
    }
}

#[test]
/// Tests that the applying the wing angles to the line force model rotates the chord vector in the
/// right way *and* that the span points are not affected by the rotation.
fn test_wing_angles() {
    let mut line_force_model = get_example_model();

    let chord_vector = line_force_model.chord_vectors_local_not_rotated[0];

    let rotation_angle = Float::to_radians(45.0);

    let rotated_chord_vector = chord_vector.rotate_around_axis(
        rotation_angle, 
        SpatialVector::from([0.0, 0.0, 1.0])
    );

    let original_span_points = line_force_model.span_points_global.clone();

    let wing_angles = vec![rotation_angle, rotation_angle];

    line_force_model.set_local_wing_angles(&wing_angles);

    let chord_vectors = &line_force_model.chord_vectors_global;
    let span_points = &line_force_model.span_points_global.clone();

    for &chord_vector in chord_vectors {
        assert_eq!(chord_vector, rotated_chord_vector);
    }

    for (span_point, original_span_point) in span_points.iter().zip(&original_span_points) {
        assert_eq!(span_point, original_span_point);
    }
}
#[test]
/// The rotation type in the builder should be used for the rigid body motion of the model
fn rotation_type_from_builder_is_used() {
    use stormath::spatial_vector::transformations::RotationType;

    let mut builder = LineForceModelBuilder::new(5);

    builder.add_wing(WingBuilder {
        section_points: vec![
            SpatialVector::from([0.0, 0.0, 0.0]),
            SpatialVector::from([0.0, 0.0, 10.0]),
        ],
        chord_vectors: vec![SpatialVector::from([1.0, 0.0, 0.0]); 2],
        section_model: SectionModel::SingleElementFoil(Foil::default()),
        ..Default::default()
    });

    builder.rotation = SpatialVector::from([0.3, -0.2, 0.5]);
    builder.rotation_type = RotationType::ZYX;

    let model = builder.build().unwrap();

    assert_eq!(model.rigid_body_motion.rotation_type, RotationType::ZYX);

    let expected_top = SpatialVector::from([0.0, 0.0, 10.0])
        .rotate(builder.rotation, RotationType::ZYX);

    let last_index = model.nr_span_lines() - 1;

    assert!((model.span_lines_global[last_index].end_point - expected_top).length() < 1e-9);
}
