// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Tests for the wing level force models: force corrections, added mass and gyroscopic moments.

use crate::test_utils::tolerance;
use super::*;

use stormath::type_aliases::Float;
use stormath::consts::PI;

use crate::common_utils::forces_and_moments::{CoordinateSystem, SectionalForcesInput};
use crate::line_force_model::corrections::force::ForceCorrection;
use crate::line_force_model::added_mass::AddedMass;
use crate::line_force_model::gyroscopic_moment::GyroscopicMoment;
use crate::section_models::rotating_cylinder::RotatingCylinder;

const CHORD_LENGTH: Float = 2.0;
const DENSITY: Float = 1.2;

/// A single wing along the z-axis, with the chord along the x-axis. The first segment is virtual.
fn wing_builder(section_model: SectionModel) -> WingBuilder {
    let chord_vector = SpatialVector::from([CHORD_LENGTH, 0.0, 0.0]);

    WingBuilder {
        section_points: vec![
            SpatialVector::from([0.0, 0.0, -2.0]),
            SpatialVector::from([0.0, 0.0, 0.0]),
            SpatialVector::from([0.0, 0.0, 10.0]),
        ],
        chord_vectors: vec![chord_vector; 3],
        section_model,
        line_segment_is_virtual: Some(vec![true, false]),
        ..Default::default()
    }
}

fn build(wing_builder: WingBuilder) -> LineForceModel {
    let mut builder = LineForceModelBuilder::new(12);

    builder.density = DENSITY;
    builder.add_wing(wing_builder);

    builder.build()
}

fn force_input(
    line_force_model: &LineForceModel,
    velocity: SpatialVector,
    acceleration: SpatialVector,
    rotation_velocity: SpatialVector
) -> SectionalForcesInput {
    let nr_span_lines = line_force_model.nr_span_lines();

    SectionalForcesInput {
        circulation_strength: vec![-3.0; nr_span_lines],
        velocity: vec![velocity; nr_span_lines],
        angles_of_attack: vec![Float::to_radians(30.0); nr_span_lines],
        section_models_internal_state: vec![0.0; nr_span_lines],
        acceleration: vec![acceleration; nr_span_lines],
        rotation_velocity,
        coordinate_system: CoordinateSystem::Global,
    }
}

fn assert_vectors_equal(a: SpatialVector, b: SpatialVector) {
    assert!(
        (a - b).length() < tolerance(1e-9, 1e-5) * (1.0 + b.length()),
        "{:?} != {:?}", a, b
    );
}

#[test]
/// The lift-induced drag correction should add a drag force equal to the factor times the square
/// of the lift coefficient from the total lift force, i.e., circulatory and viscous lift.
fn lift_induced_drag_correction() {
    let factor = 0.05;

    let without_correction = build(wing_builder(SectionModel::default()));
    let with_correction = build(WingBuilder {
        force_correction: ForceCorrection { lift_induced_drag_correction: factor },
        ..wing_builder(SectionModel::default())
    });

    let velocity = SpatialVector::from([8.0, 1.0, 0.0]);

    let input = force_input(&without_correction, velocity, SpatialVector::default(), SpatialVector::default());

    let forces_without = without_correction.sectional_forces(&input);
    let forces_with = with_correction.sectional_forces(&input);

    for i in 0..without_correction.nr_span_lines() {
        let area = without_correction.chord_lengths[i] * without_correction.span_lines_local[i].length();
        let force_factor = 0.5 * DENSITY * area * velocity.length_squared();

        let lift_force = forces_without.circulatory[i] + forces_without.viscous_lift[i];
        let lift_coefficient = lift_force.length() / force_factor;

        assert!(lift_coefficient > 0.1);

        let expected = velocity.normalize() * factor * lift_coefficient.powi(2) * force_factor;

        assert_vectors_equal(forces_with.sectional_drag[i] - forces_without.sectional_drag[i], expected);
        assert_vectors_equal(forces_with.total[i] - forces_without.total[i], expected);
    }
}

#[test]
/// The added mass force should use the reference mass of a flat plate, with separate coefficients
/// for the acceleration normal to and along the chord. Acceleration along the span gives no force.
fn added_mass_force() {
    let acceleration = SpatialVector::from([2.0, 3.0, 5.0]);

    let cases = [
        (AddedMass { normal_coefficient: 1.0, chordwise_coefficient: 0.0 }, SpatialVector::from([0.0, 3.0, 0.0])),
        (AddedMass { normal_coefficient: 1.0, chordwise_coefficient: 1.0 }, SpatialVector::from([2.0, 3.0, 0.0])),
        (AddedMass { normal_coefficient: 0.5, chordwise_coefficient: 0.1 }, SpatialVector::from([0.2, 1.5, 0.0])),
    ];

    for (added_mass, effective_acceleration) in cases {
        let line_force_model = build(WingBuilder {
            added_mass,
            ..wing_builder(SectionModel::default())
        });

        let input = force_input(&line_force_model, SpatialVector::default(), acceleration, SpatialVector::default());

        let forces = line_force_model.sectional_forces(&input);

        for i in 0..line_force_model.nr_span_lines() {
            let span_length = line_force_model.span_lines_local[i].length();
            let reference_mass = DENSITY * PI * CHORD_LENGTH.powi(2) / 4.0 * span_length;

            assert_vectors_equal(forces.added_mass[i], reference_mass * effective_acceleration);
        }
    }

    let no_added_mass = build(wing_builder(SectionModel::default()));
    let input = force_input(&no_added_mass, SpatialVector::default(), acceleration, SpatialVector::default());

    for force in no_added_mass.sectional_forces(&input).added_mass {
        assert_eq!(force, SpatialVector::default());
    }
}

#[test]
/// The integrated gyroscopic moment should equal L x omega, where L is the angular momentum of the
/// whole rotor, regardless of the virtual line elements. It should not be part of the forces.
fn gyroscopic_moment() {
    let moment_of_inertia = 1500.0;
    let revolutions_per_second = 2.5;

    let mut line_force_model = build(WingBuilder {
        gyroscopic_moment: GyroscopicMoment { moment_of_inertia },
        ..wing_builder(SectionModel::RotatingCylinder(RotatingCylinder::default()))
    });

    line_force_model.set_internal_state(&[revolutions_per_second]);

    let rotation_velocity = SpatialVector::from([0.1, -0.05, 0.0]);

    let input = force_input(&line_force_model, SpatialVector::default(), SpatialVector::default(), rotation_velocity);

    let forces = line_force_model.sectional_forces(&input);

    let integrated_forces = forces.integrate_forces(&line_force_model);
    let integrated_moments = forces.integrate_moments(&line_force_model);

    let angular_momentum = moment_of_inertia * 2.0 * PI * revolutions_per_second *
        SpatialVector::from([0.0, 0.0, 1.0]);

    let expected_moment = angular_momentum.cross(rotation_velocity);

    assert_vectors_equal(integrated_moments[0].gyroscopic, expected_moment);
    assert_vectors_equal(integrated_moments[0].total, expected_moment);

    assert_eq!(integrated_forces[0].gyroscopic, SpatialVector::default());
    assert_vectors_equal(integrated_forces[0].total, SpatialVector::default());

    for i in 0..line_force_model.nr_span_lines() {
        if line_force_model.line_segment_is_virtual[i] {
            assert_eq!(forces.gyroscopic_moment[i], SpatialVector::default());
        }
    }
}

#[test]
#[should_panic]
fn gyroscopic_moment_requires_rotating_cylinder() {
    build(WingBuilder {
        gyroscopic_moment: GyroscopicMoment { moment_of_inertia: 100.0 },
        ..wing_builder(SectionModel::default())
    });
}
