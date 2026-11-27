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
use crate::section_models::dynamic_stall::DynamicStallModelBuilder;
use crate::error::Error;

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

fn try_build(wing_builder: WingBuilder) -> Result<LineForceModel, Error> {
    let mut builder = LineForceModelBuilder::new(12);

    builder.density = DENSITY;
    builder.add_wing(wing_builder);

    builder.build()
}

fn build(wing_builder: WingBuilder) -> LineForceModel {
    try_build(wing_builder).unwrap()
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
        amount_of_stall: vec![0.0; nr_span_lines],
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
fn gyroscopic_moment_requires_rotating_cylinder() {
    assert!(try_build(WingBuilder {
        gyroscopic_moment: GyroscopicMoment { moment_of_inertia: 100.0 },
        ..wing_builder(SectionModel::default())
    }).is_err());
}

#[test]
fn dynamic_stall_from_json() {
    let wing_builder: WingBuilder = serde_json::from_str(r#"{
        "section_points": [{"x": 0.0, "y": 0.0, "z": 0.0}, {"x": 0.0, "y": 0.0, "z": 1.0}],
        "chord_vectors": [{"x": 1.0, "y": 0.0, "z": 0.0}, {"x": 1.0, "y": 0.0, "z": 0.0}],
        "section_model": {"SingleElementFoil": {}},
        "dynamic_stall": {"separation_time_constant": 4.0}
    }"#).unwrap();

    let line_force_model = build(wing_builder);

    let dynamic_stall = line_force_model.dynamic_stall[0].as_ref().unwrap();

    assert_eq!(dynamic_stall.separation_time_constant, 4.0);
    assert_eq!(dynamic_stall.segment_states.len(), line_force_model.nr_span_lines());
}

#[test]
fn dynamic_stall_with_internal_state_lag_for_rotating_cylinder() {
    let line_force_model = build(WingBuilder {
        dynamic_stall: Some(DynamicStallModelBuilder {
            internal_state_time_constant: 2.0,
            ..Default::default()
        }),
        ..wing_builder(SectionModel::RotatingCylinder(RotatingCylinder::default()))
    });

    let dynamic_stall = line_force_model.dynamic_stall[0].as_ref().unwrap();

    assert_eq!(dynamic_stall.internal_state_time_constant, 2.0);
}

#[test]
fn dynamic_stall_parameters_require_foil_section_model() {
    assert!(try_build(WingBuilder {
        dynamic_stall: Some(DynamicStallModelBuilder {
            separation_time_constant: Some(4.0),
            ..Default::default()
        }),
        ..wing_builder(SectionModel::RotatingCylinder(RotatingCylinder::default()))
    }).is_err());
}

#[test]
/// The angle of attack is the angle in the plane of the section, i.e., normal to the span line.
/// A velocity component along the span should therefore not change the angle of attack.
fn the_span_velocity_does_not_change_the_angle_of_attack() {
    let line_force_model = build(wing_builder(SectionModel::SingleElementFoil(Foil::default())));

    let nr_span_lines = line_force_model.nr_span_lines();

    for angle in [-170.0, -60.0, -10.0, 0.0, 5.0, 30.0, 120.0] {
        let angle_rad = Float::to_radians(angle);

        // The span is along the z-axis and the chord along the x-axis
        let in_plane = SpatialVector::new(angle_rad.cos(), angle_rad.sin(), 0.0) * 8.0;
        let with_span_component = in_plane + SpatialVector::new(0.0, 0.0, 5.0);

        let expected = line_force_model.angles_of_attack(
            &vec![in_plane; nr_span_lines], CoordinateSystem::Global
        );

        let calculated = line_force_model.angles_of_attack(
            &vec![with_span_component; nr_span_lines], CoordinateSystem::Global
        );

        for i in 0..nr_span_lines {
            assert!(
                (calculated[i] - expected[i]).abs() < tolerance(1e-12, 1e-5),
                "angle {}: {} with span velocity, {} without",
                angle, calculated[i].to_degrees(), expected[i].to_degrees()
            );

            assert!(
                (expected[i].to_degrees() - angle).abs() < tolerance(1e-9, 1e-3),
                "angle {}: {}", angle, expected[i].to_degrees()
            );
        }
    }

    // A velocity along the span only has no angle of attack
    let along_span = line_force_model.angles_of_attack(
        &vec![SpatialVector::new(0.0, 0.0, 3.0); nr_span_lines], CoordinateSystem::Global
    );

    assert!(along_span.iter().all(|angle| *angle == 0.0), "{:?}", along_span);
}

#[test]
/// The chord vector is projected onto the plane normal to the span line, in the same way as the
/// velocity, so that a chord that is not normal to the span line, e.g., a horizontal chord on a
/// raked span line, does not change the angle of attack.
fn a_chord_along_the_span_does_not_change_the_angle_of_attack() {
    let rake = Float::to_radians(10.0);

    // Span line in the x-z plane, raked 10 degrees backwards, with a horizontal chord
    let span_top = SpatialVector::new(10.0 * rake.sin(), 0.0, 10.0 * rake.cos());

    let line_force_model = build(WingBuilder {
        section_points: vec![SpatialVector::new(0.0, 0.0, 0.0), span_top],
        chord_vectors: vec![SpatialVector::new(CHORD_LENGTH, 0.0, 0.0); 2],
        section_model: SectionModel::SingleElementFoil(Foil::default()),
        ..Default::default()
    });

    let nr_span_lines = line_force_model.nr_span_lines();

    for angle in [-30.0, 0.0, 5.0, 20.0] {
        let angle_rad = Float::to_radians(angle);

        let velocity = SpatialVector::new(angle_rad.cos(), angle_rad.sin(), 0.0) * 8.0;

        let calculated = line_force_model.angles_of_attack(
            &vec![velocity; nr_span_lines], CoordinateSystem::Global
        );

        // The in-plane angle. The projected chord is along (cos(rake), 0, -sin(rake)), and the
        // projected velocity has the components cos(a) cos(rake) along the chord and sin(a)
        // along the y-axis. Without the projection of the chord, the angle at a = 0 was -10 deg.
        let expected = (angle_rad.tan() / rake.cos()).atan();

        for angle_i in calculated {
            assert!(
                (angle_i - expected).abs() < tolerance(1e-9, 1e-4),
                "angle {}: {} calculated, {} expected",
                angle, angle_i.to_degrees(), expected.to_degrees()
            );
        }
    }
}

#[test]
/// Virtual line elements are not part of the wing, so their area should not be included in the
/// projected area, in the same way as they are not included in the integrated forces.
fn the_projected_area_excludes_virtual_line_elements() {
    let line_force_model = build(wing_builder(SectionModel::SingleElementFoil(Foil::default())));

    // Only the segment from z = 0 to z = 10 is real
    let real_area = CHORD_LENGTH * 10.0;

    let areas = line_force_model.projected_areas();

    assert_eq!(areas.len(), 1);
    assert!((areas[0] - real_area).abs() < tolerance(1e-9, 1e-4), "area: {}", areas[0]);
    assert!(
        (line_force_model.total_projected_area() - real_area).abs() < tolerance(1e-9, 1e-4),
        "total area: {}", line_force_model.total_projected_area()
    );

    // The aspect ratio of the real wing: a span of 10 and an area of 20
    let aspect_ratios = line_force_model.aspect_ratios();

    assert!((aspect_ratios[0] - 5.0).abs() < tolerance(1e-9, 1e-4), "aspect ratio: {}", aspect_ratios[0]);
}
