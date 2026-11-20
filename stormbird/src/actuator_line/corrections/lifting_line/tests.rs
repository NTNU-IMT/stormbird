// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use super::*;

use crate::line_force_model::builder::LineForceModelBuilder;
use crate::line_force_model::builder::single_wing::WingBuilder;
use crate::section_models::SectionModel;
use crate::section_models::foil::Foil;
use crate::test_utils::tolerance;

use super::panel_wake::core_lengths_along_wake;

const CHORD_LENGTH: Float = 5.0;
const VISCOUS_CORE_LENGTH_FACTOR: Float = 0.6;

/// A line force model with straight wings along the z-axis, with the given x and y positions
fn line_force_model(wing_positions: &[[Float; 2]], nr_sections: usize) -> LineForceModel {
    let mut builder = LineForceModelBuilder::new(nr_sections);

    for position in wing_positions {
        builder.add_wing(WingBuilder {
            section_points: vec![
                SpatialVector::from([position[0], position[1], 2.0]),
                SpatialVector::from([position[0], position[1], 32.0]),
            ],
            chord_vectors: vec![SpatialVector::from([CHORD_LENGTH, 0.0, 0.0]); 2],
            section_model: SectionModel::SingleElementFoil(Foil::default()),
            ..Default::default()
        });
    }

    builder.build().unwrap()
}

/// A single straight wing along the z-axis, with uniform circulation and a uniform velocity
/// along the x-axis
fn single_wing() -> (LineForceModel, Vec<SpatialVector>, Vec<Float>) {
    let line_force_model = line_force_model(&[[0.0, 0.0]], 16);
    let nr_span_lines = line_force_model.nr_span_lines();

    let velocity = vec![SpatialVector::from([8.0, 0.0, 0.0]); nr_span_lines];
    let circulation_strength = vec![10.0; nr_span_lines];

    (line_force_model, velocity, circulation_strength)
}

fn straight_with_extensions(virtual_extension_factors: [Float; 2]) -> CorrectionWakeBuilder {
    CorrectionWakeBuilder::Straight(StraightWakeBuilder {
        virtual_extension_factors,
        ..Default::default()
    })
}

fn straight_panels(reference_core_evolution_length_factor: Option<Float>) -> CorrectionWakeBuilder {
    CorrectionWakeBuilder::StraightPanels(StraightPanelWakeBuilder {
        reference_core_evolution_length_factor,
        ..Default::default()
    })
}

/// Updates a correction that does not depend on the time or the circulation history, i.e., not
/// the dynamic wake
fn update_static(correction: &mut LiftingLineCorrection, line_force_model: &LineForceModel, velocity: &[SpatialVector]) {
    let zero_strength = vec![0.0; line_force_model.nr_span_lines()];

    correction.update(line_force_model, velocity, &zero_strength, &zero_strength, 0.0, 1.0);
}

/// Builds and updates the correction, and returns the velocity correction
fn split_correction(
    builder: LiftingLineCorrectionBuilder,
    line_force_model: &LineForceModel,
    velocity: &[SpatialVector],
    reference_circulation_strength: &[Float],
    projected_circulation_strength: &[Float],
) -> Vec<SpatialVector> {
    let mut correction = builder.build(VISCOUS_CORE_LENGTH_FACTOR, line_force_model).unwrap();

    update_static(&mut correction, line_force_model, velocity);

    correction.velocity_correction(
        line_force_model,
        velocity,
        reference_circulation_strength,
        projected_circulation_strength,
        0.0
    )
}

fn correction(
    builder: LiftingLineCorrectionBuilder,
    line_force_model: &LineForceModel,
    velocity: &[SpatialVector],
    circulation_strength: &[Float],
) -> Vec<SpatialVector> {
    split_correction(builder, line_force_model, velocity, circulation_strength, circulation_strength)
}

/// Time step for the dynamic wake. With the velocity of 8 m/s in the tests, the points move half a
/// chord length in each time step.
const TIME_STEP: Float = 0.5 * CHORD_LENGTH / 8.0;

fn dynamic(builder: DynamicCorrectionWakeBuilder) -> CorrectionWakeBuilder {
    CorrectionWakeBuilder::Dynamic(builder)
}

fn dynamic_with_nr_panels(nr_panels_per_line_element: usize) -> CorrectionWakeBuilder {
    dynamic(DynamicCorrectionWakeBuilder {
        nr_panels_per_line_element,
        ..Default::default()
    })
}

fn dynamic_wake(correction: &LiftingLineCorrection) -> &DynamicCorrectionWake {
    let CorrectionWake::Dynamic(wake) = &correction.wake else {
        panic!("Expected a dynamic wake");
    };

    wake
}

/// Updates the correction at the given time step index, with the same circulation for the
/// reference and the projected forces
fn update_at_step(
    correction: &mut LiftingLineCorrection,
    line_force_model: &LineForceModel,
    velocity: &[SpatialVector],
    previous_circulation_strength: &[Float],
    time_step_index: usize,
) {
    correction.update(
        line_force_model,
        velocity,
        previous_circulation_strength,
        previous_circulation_strength,
        time_step_index as Float * TIME_STEP,
        TIME_STEP
    );
}

fn max_length(vectors: &[SpatialVector]) -> Float {
    vectors.iter().map(|v| v.length()).fold(0.0, Float::max)
}

fn max_difference(a: &[SpatialVector], b: &[SpatialVector]) -> Float {
    a.iter().zip(b).map(|(a, b)| (*a - *b).length()).fold(0.0, Float::max)
}

#[test]
fn reference_and_projected_circulation_are_used_in_the_right_models() {
    let (line_force_model, velocity, circulation_strength) = single_wing();
    let zero_circulation_strength = vec![0.0; circulation_strength.len()];

    // A tapered projected circulation, so that it differs from the reference
    let nr_span_lines = circulation_strength.len();
    let projected_circulation_strength: Vec<Float> = (0..nr_span_lines).map(|i| {
        let s = (i as Float + 0.5) / nr_span_lines as Float - 0.5;
        circulation_strength[i] * (1.0 - 4.0 * s * s)
    }).collect();

    for wake in [straight_with_extensions([0.4, 1.0]), straight_panels(Some(10.0))] {
        for include_wing_interactions in [false, true] {
            let settings = || LiftingLineCorrectionBuilder {
                wake: wake.clone(),
                include_wing_interactions,
                ..Default::default()
            };

            let reference_only = split_correction(
                settings(), &line_force_model, &velocity,
                &circulation_strength, &zero_circulation_strength
            );

            let viscous_only = split_correction(
                settings(), &line_force_model, &velocity,
                &zero_circulation_strength, &circulation_strength
            );

            // The reference model, with the small viscous core, is close to singular at the ends,
            // while the model with the viscous core is not
            assert!(reference_only[0].length() > 2.0 * viscous_only[0].length());

            // The correction is linear in the two circulation distributions
            let split = split_correction(
                settings(), &line_force_model, &velocity,
                &circulation_strength, &projected_circulation_strength
            );

            let projected_only = split_correction(
                settings(), &line_force_model, &velocity,
                &zero_circulation_strength, &projected_circulation_strength
            );

            for i in 0..split.len() {
                let expected = reference_only[i] + projected_only[i];

                assert!((split[i] - expected).length() < 1e-9 * (1.0 + expected.length()));
            }
        }
    }
}

#[test]
fn max_correction_ratio_limits_the_correction() {
    let (line_force_model, velocity, circulation_strength) = single_wing();

    let max_correction_ratio = 0.01;

    let limited = correction(
        LiftingLineCorrectionBuilder {
            max_correction_ratio: Some(max_correction_ratio),
            ..Default::default()
        },
        &line_force_model, &velocity, &circulation_strength
    );

    let unlimited = correction(
        LiftingLineCorrectionBuilder::default(),
        &line_force_model, &velocity, &circulation_strength
    );

    for i in 0..limited.len() {
        let max_magnitude = max_correction_ratio * velocity[i].length();

        assert!(limited[i].length() <= max_magnitude * (1.0 + 1e-9));

        // The direction is kept
        assert!(limited[i].normalize().dot(unlimited[i].normalize()) > 1.0 - tolerance(1e-9, 1e-6));
    }

    // The limit is active at the ends, where the correction is largest
    assert!(unlimited[0].length() > max_correction_ratio * velocity[0].length());
}

#[test]
fn virtual_extensions_move_the_tip_vortices() {
    let (line_force_model, velocity, circulation_strength) = single_wing();

    let without_extensions = correction(
        LiftingLineCorrectionBuilder::default(),
        &line_force_model, &velocity, &circulation_strength
    );

    let with_extensions = correction(
        LiftingLineCorrectionBuilder {
            wake: straight_with_extensions([20.0, 20.0]),
            ..Default::default()
        },
        &line_force_model, &velocity, &circulation_strength
    );

    let last = without_extensions.len() - 1;

    // The reference model is close to singular at the outermost control points without the
    // extensions, as the tip vortex is only half a line element away
    for index in [0, last] {
        assert!(with_extensions[index].length() < 0.2 * without_extensions[index].length());
    }
}

#[test]
fn isolated_and_all_wings_corrections_are_equal_for_a_single_wing() {
    let (line_force_model, velocity, circulation_strength) = single_wing();

    // Different reference and projected circulation, to check that both versions use them in
    // the same way
    let projected_circulation_strength: Vec<Float> = circulation_strength.iter()
        .enumerate()
        .map(|(i, value)| value * (0.5 + 0.03 * i as Float))
        .collect();

    for wake in [straight_with_extensions([0.4, 1.0]), straight_panels(Some(10.0))] {
        let settings = |include_wing_interactions| LiftingLineCorrectionBuilder {
            wake: wake.clone(),
            symmetry_condition: SymmetryCondition::Z,
            include_wing_interactions,
            ..Default::default()
        };

        let isolated = split_correction(
            settings(false), &line_force_model, &velocity,
            &circulation_strength, &projected_circulation_strength
        );
        let all_wings = split_correction(
            settings(true), &line_force_model, &velocity,
            &circulation_strength, &projected_circulation_strength
        );

        for (u_isolated, u_all_wings) in isolated.iter().zip(&all_wings) {
            assert!((*u_isolated - *u_all_wings).length() < 1e-9 * (1.0 + u_isolated.length()));
        }
    }
}

#[test]
/// With a constant reference core length, the straight panel wake represents the same vortex
/// system as the horseshoe vortices, so the corrections should be close.
///
/// They are not exactly equal, as the viscous core term of a vortex line depends on the distance
/// to the closest point on the line segment. A trailing vortex that is split into several panels
/// is therefore damped less by the viscous core than a single long vortex line, when the point is
/// close to the start of the trailing vortex. The largest difference is at the outermost control
/// points, close to the tip vortices of the reference model.
fn straight_panels_with_constant_core_is_close_to_straight() {
    let line_force_model = line_force_model(&[[0.0, 0.0], [40.0, 15.0]], 12);
    let nr_span_lines = line_force_model.nr_span_lines();

    let velocity = vec![SpatialVector::from([8.0, 1.0, 0.0]); nr_span_lines];
    let circulation_strength: Vec<Float> = (0..nr_span_lines)
        .map(|i| 10.0 + (i as Float * 0.5).sin())
        .collect();

    for include_wing_interactions in [false, true] {
        let settings = |wake| LiftingLineCorrectionBuilder {
            wake,
            include_wing_interactions,
            ..Default::default()
        };

        let straight = correction(
            settings(CorrectionWakeBuilder::default()),
            &line_force_model, &velocity, &circulation_strength
        );

        let panels = correction(
            settings(straight_panels(None)),
            &line_force_model, &velocity, &circulation_strength
        );

        let relative_difference = max_difference(&straight, &panels) / max_length(&straight);

        dbg!(include_wing_interactions, relative_difference);

        assert!(relative_difference < 0.05);
    }
}

#[test]
/// The tip vortices of an upstream wing pass close to the ends of a downstream wing. When the
/// reference core length grows to the viscous core length along the wake, the two wake models
/// are equal at the downstream wing, so the correction from the upstream wing should vanish.
fn reference_core_evolution_removes_the_correction_from_upstream_wings() {
    let line_force_model = line_force_model(&[[0.0, 0.0], [10.0 * CHORD_LENGTH, 0.0]], 12);
    let nr_span_lines = line_force_model.nr_span_lines();
    let downstream_wing = line_force_model.wing_indices[1].clone();

    let velocity = vec![SpatialVector::from([8.0, 0.0, 0.0]); nr_span_lines];
    let circulation_strength = vec![10.0; nr_span_lines];

    // The correction from the other wings is the difference between the correction with and
    // without wing interactions
    let correction_from_other_wings = |wake: CorrectionWakeBuilder| -> Vec<SpatialVector> {
        let settings = |include_wing_interactions| LiftingLineCorrectionBuilder {
            wake: wake.clone(),
            include_wing_interactions,
            ..Default::default()
        };

        let all_wings = correction(settings(true), &line_force_model, &velocity, &circulation_strength);
        let isolated = correction(settings(false), &line_force_model, &velocity, &circulation_strength);

        all_wings.iter().zip(&isolated).map(|(a, b)| *a - *b).collect()
    };

    let constant_core = correction_from_other_wings(straight_panels(None));
    let growing_core = correction_from_other_wings(straight_panels(Some(2.0)));

    let constant_core_downstream = max_length(&constant_core[downstream_wing.clone()]);
    let growing_core_downstream = max_length(&growing_core[downstream_wing]);

    dbg!(constant_core_downstream, growing_core_downstream);

    assert!(constant_core_downstream > 1e-3);
    assert!(growing_core_downstream < 0.01 * constant_core_downstream);
}

#[test]
fn core_lengths_grow_to_the_end_value_along_the_wake() {
    let (line_force_model, velocity, _) = single_wing();

    let mut correction = LiftingLineCorrectionBuilder {
        wake: straight_panels(Some(4.0)),
        ..Default::default()
    }.build(VISCOUS_CORE_LENGTH_FACTOR, &line_force_model).unwrap();

    update_static(&mut correction, &line_force_model, &velocity);

    let CorrectionWake::StraightPanels(wake) = &correction.wake else {
        panic!("Expected a straight panel wake");
    };

    let settings = &correction.settings;
    let reference = &wake.wakes.reference;
    let indices = &reference.indices;

    let start_value = settings.reference_core_length;
    let end_value = settings.viscous_core_length;
    let evolution_length = 4.0 * CHORD_LENGTH;

    let core_lengths = core_lengths_along_wake(reference, start_value, end_value, evolution_length);

    // The values used in the wake are the same as the function gives
    assert_eq!(core_lengths, reference.panels_viscous_core_length);

    for i_span in 0..indices.nr_panels_along_span {
        let mut previous = 0.0;

        for i_stream in 0..indices.nr_panels_per_line_element {
            let value = core_lengths[indices.panel_index(i_stream, i_span)];

            assert!(value >= previous && value >= start_value && value <= end_value * (1.0 + tolerance(1e-12, 1e-6)));

            previous = value;
        }

        // The first panel is short, so the value is closer to the start value than the end value,
        // while the last panels are far beyond the evolution length
        let first = core_lengths[indices.panel_index(0, i_span)];
        let last = core_lengths[indices.panel_index(indices.nr_panels_per_line_element - 1, i_span)];

        assert!(first < 0.2 * end_value);
        assert!((last - end_value).abs() < tolerance(1e-12, 1e-6) * end_value);
    }

    // The viscous wake has a constant core length
    assert!(wake.wakes.viscous.panels_viscous_core_length.iter().all(|value| *value == end_value));
}

#[test]
fn straight_panel_geometry_follows_the_sampled_velocity() {
    let (line_force_model, _, _) = single_wing();
    let nr_span_lines = line_force_model.nr_span_lines();

    let direction = SpatialVector::from([3.0, 4.0, 0.0]).normalize();
    let velocity = vec![8.0 * direction; nr_span_lines];

    let builder = StraightPanelWakeBuilder::default();

    let mut correction = LiftingLineCorrectionBuilder {
        wake: CorrectionWakeBuilder::StraightPanels(builder.clone()),
        ..Default::default()
    }.build(VISCOUS_CORE_LENGTH_FACTOR, &line_force_model).unwrap();

    update_static(&mut correction, &line_force_model, &velocity);

    let CorrectionWake::StraightPanels(wake) = &correction.wake else {
        panic!("Expected a straight panel wake");
    };

    let reference = &wake.wakes.reference;
    let indices = &reference.indices;
    let nr_panels = indices.nr_panels_per_line_element;

    for i_span in 0..indices.nr_points_along_span {
        let start = line_force_model.span_points_global[i_span];

        let first = reference.points[indices.point_index(1, i_span)];
        let last = reference.points[indices.point_index(nr_panels, i_span)];

        let length_tolerance = tolerance(1e-9, 1e-3);

        assert!(((first - start).length() - builder.first_panel_relative_length * CHORD_LENGTH).abs() < length_tolerance);
        assert!(((last - start).length() - builder.wake_length_factor * CHORD_LENGTH).abs() < length_tolerance);
        assert!((last - start).normalize().dot(direction) > 1.0 - tolerance(1e-12, 1e-6));

        // The viscous wake has the same geometry
        assert_eq!(last, wake.wakes.viscous.points[indices.point_index(nr_panels, i_span)]);
    }
}

#[test]
/// With a uniform velocity and a constant circulation, the dynamic wake becomes a straight panel
/// wake with the same circulation in all panels, once the circulation has been streamed through
/// the whole wake. The correction should then be close to the straight wake with the same length.
/// See `straight_panels_with_constant_core_is_close_to_straight` for why they are not equal.
fn dynamic_correction_converges_to_straight() {
    let line_force_model = line_force_model(&[[0.0, 0.0], [40.0, 15.0]], 12);
    let nr_span_lines = line_force_model.nr_span_lines();

    let velocity = vec![SpatialVector::from([8.0, 0.0, 0.0]); nr_span_lines];
    let circulation_strength: Vec<Float> = (0..nr_span_lines)
        .map(|i| 10.0 + (i as Float * 0.5).sin())
        .collect();

    let builder = DynamicCorrectionWakeBuilder {
        nr_panels_per_line_element: 50,
        ..Default::default()
    };

    // First panel, the rows that move half a chord per time step, and the last panel
    let wake_length_factor = builder.first_panel_relative_length +
        0.5 * (builder.nr_panels_per_line_element - 2) as Float +
        builder.last_panel_relative_length;

    for include_wing_interactions in [false, true] {
        let settings = |wake| LiftingLineCorrectionBuilder {
            wake,
            include_wing_interactions,
            ..Default::default()
        };

        let straight = correction(
            settings(CorrectionWakeBuilder::Straight(StraightWakeBuilder {
                wake_length_factor,
                ..Default::default()
            })),
            &line_force_model, &velocity, &circulation_strength
        );

        let mut dynamic_correction = settings(dynamic(builder.clone()))
            .build(VISCOUS_CORE_LENGTH_FACTOR, &line_force_model).unwrap();

        for time_step_index in 0..=(2 * builder.nr_panels_per_line_element) {
            update_at_step(
                &mut dynamic_correction,
                &line_force_model,
                &velocity,
                &circulation_strength,
                time_step_index
            );
        }

        let dynamic = dynamic_correction.velocity_correction(
            &line_force_model,
            &velocity,
            &circulation_strength,
            &circulation_strength,
            0.0
        );

        let relative_difference = max_difference(&straight, &dynamic) / max_length(&straight);

        dbg!(include_wing_interactions, relative_difference);

        assert!(relative_difference < 0.05);
    }
}

#[test]
/// The strength of row k in the wake is the circulation from k time steps earlier, with the
/// reference circulation in the reference wake and the projected circulation in the viscous wake.
fn dynamic_wake_strength_is_the_circulation_history() {
    let (line_force_model, velocity, _) = single_wing();
    let nr_span_lines = line_force_model.nr_span_lines();

    let mut correction = LiftingLineCorrectionBuilder {
        wake: dynamic_with_nr_panels(10),
        ..Default::default()
    }.build(VISCOUS_CORE_LENGTH_FACTOR, &line_force_model).unwrap();

    // The circulation that results from the time step with the given index
    let reference_circulation = |time_step_index: usize| vec![1.0 + time_step_index as Float; nr_span_lines];
    let projected_circulation = |time_step_index: usize| vec![2.0 * (1.0 + time_step_index as Float); nr_span_lines];

    let zero_strength = vec![0.0; nr_span_lines];

    let last_time_step_index = 5;

    for time_step_index in 0..=last_time_step_index {
        let (previous_reference, previous_projected) = if time_step_index == 0 {
            (zero_strength.clone(), zero_strength.clone())
        } else {
            (reference_circulation(time_step_index - 1), projected_circulation(time_step_index - 1))
        };

        correction.update(
            &line_force_model,
            &velocity,
            &previous_reference,
            &previous_projected,
            time_step_index as Float * TIME_STEP,
            TIME_STEP
        );
    }

    let wakes = &dynamic_wake(&correction).wakes;
    let indices = &wakes.reference.indices;

    for row in 1..indices.nr_panels_per_line_element {
        let (expected_reference, expected_projected) = if row <= last_time_step_index {
            (
                reference_circulation(last_time_step_index - row)[0],
                projected_circulation(last_time_step_index - row)[0]
            )
        } else {
            (0.0, 0.0)
        };

        for i_span in 0..indices.nr_panels_along_span {
            let index = indices.panel_index(row, i_span);

            assert_eq!(wakes.reference.strengths[index], expected_reference, "row {row}");
            assert_eq!(wakes.viscous.strengths[index], expected_projected, "row {row}");
        }
    }
}

#[test]
/// The wake is only updated in the first call with a new time, so that outer iterations in the CFD
/// solver do not move the wake
fn dynamic_wake_is_updated_once_per_time_step() {
    let (line_force_model, velocity, circulation_strength) = single_wing();

    let mut correction = LiftingLineCorrectionBuilder {
        wake: dynamic_with_nr_panels(10),
        ..Default::default()
    }.build(VISCOUS_CORE_LENGTH_FACTOR, &line_force_model).unwrap();

    update_at_step(&mut correction, &line_force_model, &velocity, &circulation_strength, 0);
    update_at_step(&mut correction, &line_force_model, &velocity, &circulation_strength, 1);

    let points = dynamic_wake(&correction).wakes.reference.points.clone();
    let strengths = dynamic_wake(&correction).wakes.reference.strengths.clone();

    // New velocity and circulation, but the same time
    let other_velocity = vec![SpatialVector::from([4.0, 4.0, 0.0]); velocity.len()];
    let other_circulation = vec![3.0; circulation_strength.len()];

    for wake_point_index in 0..points.len() {
        correction.set_wake_point_velocity(wake_point_index, other_velocity[0]);
    }

    update_at_step(&mut correction, &line_force_model, &other_velocity, &other_circulation, 1);

    assert_eq!(points, dynamic_wake(&correction).wakes.reference.points);
    assert_eq!(strengths, dynamic_wake(&correction).wakes.reference.strengths);

    // A new time step moves the wake
    update_at_step(&mut correction, &line_force_model, &other_velocity, &other_circulation, 2);

    assert_ne!(points, dynamic_wake(&correction).wakes.reference.points);
}

#[test]
/// The first free row of points follows the sampled velocity at the control points, and the rest
/// of the points move with the sampled velocity at the wake points.
fn dynamic_wake_follows_the_sampled_wake_point_velocity() {
    let (line_force_model, velocity, circulation_strength) = single_wing();

    let builder = DynamicCorrectionWakeBuilder {
        nr_panels_per_line_element: 10,
        ..Default::default()
    };

    let mut correction = LiftingLineCorrectionBuilder {
        wake: dynamic(builder.clone()),
        ..Default::default()
    }.build(VISCOUS_CORE_LENGTH_FACTOR, &line_force_model).unwrap();

    update_at_step(&mut correction, &line_force_model, &velocity, &circulation_strength, 0);

    let nr_points = correction.wake_points_to_sample().len();
    let wake_point_velocity = SpatialVector::from([8.0, 2.0, 0.0]);

    // All the points have been moved with the new velocity after one time step per panel
    for time_step_index in 1..=(builder.nr_panels_per_line_element + 1) {
        for index in 0..nr_points {
            correction.set_wake_point_velocity(index, wake_point_velocity);
        }

        update_at_step(&mut correction, &line_force_model, &velocity, &circulation_strength, time_step_index);
    }

    let wakes = &dynamic_wake(&correction).wakes;
    let indices = &wakes.reference.indices;
    let points = &wakes.reference.points;

    let length_tolerance = tolerance(1e-9, 1e-3);

    for i_span in 0..indices.nr_points_along_span {
        let span_point = line_force_model.span_points_global[i_span];
        let first_free_point = points[indices.point_index(1, i_span)];

        // The first free row follows the control point velocity along the x-axis
        let expected_first = span_point +
            SpatialVector::from([builder.first_panel_relative_length * CHORD_LENGTH, 0.0, 0.0]);

        assert!((first_free_point - expected_first).length() < length_tolerance);

        // The rest move with the wake point velocity, one time step per row. The last row is set
        // from the length of the last panel.
        for i_stream in 2..indices.nr_points_per_line_element - 1 {
            let expected = first_free_point + (i_stream - 1) as Float * TIME_STEP * wake_point_velocity;

            assert!((points[indices.point_index(i_stream, i_span)] - expected).length() < length_tolerance);
        }
    }

    // The viscous wake has the same geometry
    assert_eq!(wakes.reference.points, wakes.viscous.points);

    // The velocity stored in the wakes is the sampled velocity, with no induced velocities from
    // the wakes themselves, also for the first row of points
    for wake in [&wakes.reference, &wakes.viscous] {
        assert!(wake.velocity_at_points.iter().all(|velocity| *velocity == wake_point_velocity));
    }
}

#[test]
fn dynamic_wake_points_to_sample() {
    let (line_force_model, velocity, circulation_strength) = single_wing();

    let mut correction = LiftingLineCorrectionBuilder {
        wake: dynamic_with_nr_panels(10),
        ..Default::default()
    }.build(VISCOUS_CORE_LENGTH_FACTOR, &line_force_model).unwrap();

    // The geometry is set in the first update
    assert!(correction.wake_points_to_sample().is_empty());

    update_at_step(&mut correction, &line_force_model, &velocity, &circulation_strength, 0);

    let nr_points = (line_force_model.nr_span_lines() + 1) * 11;

    assert_eq!(correction.wake_points_to_sample().len(), nr_points);
    assert_eq!(correction.wake_points_to_sample(), &dynamic_wake(&correction).wakes.reference.points[..]);

    // The other wakes have no points to sample
    let mut straight = LiftingLineCorrectionBuilder::default()
        .build(VISCOUS_CORE_LENGTH_FACTOR, &line_force_model).unwrap();

    update_static(&mut straight, &line_force_model, &velocity);

    assert!(straight.wake_points_to_sample().is_empty());
}

#[test]
/// Points outside the CFD domain can give values that are not finite. They are ignored, so that the
/// previous value is kept.
fn non_finite_wake_point_velocities_are_ignored() {
    let (line_force_model, velocity, circulation_strength) = single_wing();

    let mut correction = LiftingLineCorrectionBuilder {
        wake: dynamic_with_nr_panels(10),
        ..Default::default()
    }.build(VISCOUS_CORE_LENGTH_FACTOR, &line_force_model).unwrap();

    update_at_step(&mut correction, &line_force_model, &velocity, &circulation_strength, 0);

    let finite_velocity = SpatialVector::from([7.0, 1.0, 0.0]);

    correction.set_wake_point_velocity(3, finite_velocity);

    for invalid_velocity in [
        SpatialVector::from([Float::NAN, 0.0, 0.0]),
        SpatialVector::from([0.0, Float::INFINITY, 0.0]),
        SpatialVector::from([0.0, 0.0, Float::NEG_INFINITY]),
    ] {
        correction.set_wake_point_velocity(3, invalid_velocity);

        assert_eq!(dynamic_wake(&correction).wake_points_velocity[3], finite_velocity);
    }
}

#[test]
/// The panel wakes can be exported as VTK, with the geometry and the strength of the panels, and
/// the sampled velocity at the points for the dynamic wake. The straight wake has no panels.
fn correction_wake_can_be_exported_as_vtk() {
    let (line_force_model, velocity, _) = single_wing();
    let nr_span_lines = line_force_model.nr_span_lines();

    let reference_circulation = vec![3.0; nr_span_lines];
    let projected_circulation = vec![2.0; nr_span_lines];
    let nr_panels = 10;

    let vtk_text = |correction: &LiftingLineCorrection| -> Option<String> {
        correction.wake_as_vtk(&reference_circulation, &projected_circulation, false)
            .map(|content| String::from_utf8(content).unwrap())
    };

    let build = |wake| LiftingLineCorrectionBuilder { wake, ..Default::default() }
        .build(VISCOUS_CORE_LENGTH_FACTOR, &line_force_model).unwrap();

    // The straight wake has nothing to export
    let mut straight = build(CorrectionWakeBuilder::default());
    update_static(&mut straight, &line_force_model, &velocity);

    assert!(vtk_text(&straight).is_none());

    let mut straight_panels = build(CorrectionWakeBuilder::StraightPanels(StraightPanelWakeBuilder {
        nr_panels_per_line_element: nr_panels,
        ..Default::default()
    }));

    let mut dynamic = build(dynamic_with_nr_panels(nr_panels));

    // The geometry is set in the first update
    assert!(vtk_text(&straight_panels).is_none());
    assert!(vtk_text(&dynamic).is_none());

    update_static(&mut straight_panels, &line_force_model, &velocity);
    update_at_step(&mut dynamic, &line_force_model, &velocity, &reference_circulation, 0);

    let nr_points = (nr_span_lines + 1) * (nr_panels + 1);
    let nr_polygons = nr_span_lines * nr_panels;

    for (correction, has_point_data) in [(&straight_panels, false), (&dynamic, true)] {
        let text = vtk_text(correction).unwrap();

        assert!(text.contains(&format!("POINTS {} float", nr_points)));
        assert!(text.contains(&format!("POLYGONS {} {}", nr_polygons, 5 * nr_polygons)));

        for name in ["reference_strength", "projected_strength", "reference_core_length", "viscous_core_length"] {
            assert!(text.contains(&format!("SCALARS {} float 1", name)), "{name}");
        }

        assert_eq!(text.contains("VECTORS sampled_velocity float"), has_point_data);

        // The first row of panels has the given circulation
        let reference_values: Vec<Float> = text.split("SCALARS reference_strength float 1\nLOOKUP_TABLE default\n")
            .nth(1).unwrap()
            .lines().take(nr_polygons)
            .map(|line| line.parse().unwrap())
            .collect();

        assert!(reference_values[..nr_span_lines].iter().all(|value| *value == 3.0));
    }

    let binary = dynamic.wake_as_vtk(&reference_circulation, &projected_circulation, true).unwrap();

    assert!(binary.starts_with(b"# vtk DataFile Version 3.0"));
}

#[test]
fn invalid_settings_are_reported() {
    let (line_force_model, _, _) = single_wing();

    let invalid_builders = [
        LiftingLineCorrectionBuilder {
            reference_core_length_factor: 0.0,
            ..Default::default()
        },
        LiftingLineCorrectionBuilder {
            wake: straight_with_extensions([-1.0, 0.0]),
            ..Default::default()
        },
        LiftingLineCorrectionBuilder {
            wake: CorrectionWakeBuilder::StraightPanels(StraightPanelWakeBuilder {
                nr_panels_per_line_element: 1,
                ..Default::default()
            }),
            ..Default::default()
        },
        LiftingLineCorrectionBuilder {
            wake: CorrectionWakeBuilder::StraightPanels(StraightPanelWakeBuilder {
                first_panel_relative_length: 200.0,
                ..Default::default()
            }),
            ..Default::default()
        },
        LiftingLineCorrectionBuilder {
            wake: straight_panels(Some(0.0)),
            ..Default::default()
        },
        LiftingLineCorrectionBuilder {
            wake: dynamic_with_nr_panels(1),
            ..Default::default()
        },
        LiftingLineCorrectionBuilder {
            wake: dynamic(DynamicCorrectionWakeBuilder {
                first_panel_relative_length: 0.0,
                ..Default::default()
            }),
            ..Default::default()
        },
        LiftingLineCorrectionBuilder {
            wake: dynamic(DynamicCorrectionWakeBuilder {
                last_panel_relative_length: -1.0,
                ..Default::default()
            }),
            ..Default::default()
        },
        LiftingLineCorrectionBuilder {
            wake: dynamic(DynamicCorrectionWakeBuilder {
                shape_damping_factor: 1.5,
                ..Default::default()
            }),
            ..Default::default()
        },
        LiftingLineCorrectionBuilder {
            wake: dynamic(DynamicCorrectionWakeBuilder {
                reference_core_evolution_length_factor: Some(-2.0),
                ..Default::default()
            }),
            ..Default::default()
        },
    ];

    for builder in invalid_builders {
        assert!(builder.check_settings().is_err());
        assert!(builder.build(VISCOUS_CORE_LENGTH_FACTOR, &line_force_model).is_err());
    }
}

#[test]
/// The settings can be read from JSON, with defaults for all fields
fn settings_from_json() {
    let builder: LiftingLineCorrectionBuilder = serde_json::from_str(r#"{
        "wake": {"StraightPanels": {"reference_core_evolution_length_factor": 3.0}},
        "symmetry_condition": "Z"
    }"#).unwrap();

    let CorrectionWakeBuilder::StraightPanels(wake) = &builder.wake else {
        panic!("Expected a straight panel wake");
    };

    assert_eq!(wake.reference_core_evolution_length_factor, Some(3.0));
    assert_eq!(wake.nr_panels_per_line_element, 50);

    let builder: LiftingLineCorrectionBuilder = serde_json::from_str(r#"{
        "wake": {"Dynamic": {"shape_damping_factor": 0.2}}
    }"#).unwrap();

    let CorrectionWakeBuilder::Dynamic(wake) = &builder.wake else {
        panic!("Expected a dynamic wake");
    };

    assert_eq!(wake.shape_damping_factor, 0.2);
    assert_eq!(wake.nr_panels_per_line_element, 100);
    assert_eq!(wake.first_panel_relative_length, 0.75);

    // The output from stormbird_setup, with all fields
    let builder: LiftingLineCorrectionBuilder = serde_json::from_str(r#"{"wake":{"Dynamic":{
        "nr_panels_per_line_element":100,"first_panel_relative_length":0.75,
        "last_panel_relative_length":1.0,"shape_damping_factor":0.2,
        "reference_core_evolution_length_factor":4.0,"far_field_ratio":5.0}},
        "symmetry_condition":"NoSymmetry","reference_core_length_factor":100.0,
        "include_wing_interactions":false}"#).unwrap();

    assert!(matches!(builder.wake, CorrectionWakeBuilder::Dynamic(_)));

    let builder: LiftingLineCorrectionBuilder = serde_json::from_str("{}").unwrap();

    assert!(matches!(builder.wake, CorrectionWakeBuilder::Straight(_)));
}

#[test]
#[ignore]
/// Times the update and the evaluation of the correction for three wings with 40 line elements
/// each. Run with:
///
/// ```text
/// cargo test --release lifting_line_correction_benchmark -- --ignored --nocapture
/// cargo test --release --features parallel lifting_line_correction_benchmark -- --ignored --nocapture
/// ```
fn lifting_line_correction_benchmark() {
    use std::time::Instant;

    let line_force_model = line_force_model(&[[-30.0, 10.0], [0.0, -10.0], [30.0, 10.0]], 40);
    let nr_span_lines = line_force_model.nr_span_lines();

    let velocity = vec![SpatialVector::from([8.0, 2.0, 0.0]); nr_span_lines];
    let circulation_strength: Vec<Float> = (0..nr_span_lines)
        .map(|i| 10.0 + (i as Float * 0.3).sin())
        .collect();

    let wakes = [
        ("Straight", CorrectionWakeBuilder::default()),
        ("StraightPanels, 50 panels", straight_panels(Some(5.0))),
        ("Dynamic, 100 panels", dynamic(DynamicCorrectionWakeBuilder {
            reference_core_evolution_length_factor: Some(5.0),
            ..Default::default()
        })),
    ];

    println!();

    for (name, wake) in wakes {
        for include_wing_interactions in [false, true] {
            let mut correction = LiftingLineCorrectionBuilder {
                wake: wake.clone(),
                symmetry_condition: SymmetryCondition::Z,
                include_wing_interactions,
                ..Default::default()
            }.build(VISCOUS_CORE_LENGTH_FACTOR, &line_force_model).unwrap();

            let nr_calls = 20;
            let mut time = 0.0;

            // Fills the dynamic wake with circulation, as panels with zero strength are skipped
            for _ in 0..=DynamicCorrectionWakeBuilder::default().nr_panels_per_line_element {
                time += TIME_STEP;

                correction.update(
                    &line_force_model,
                    &velocity,
                    &circulation_strength,
                    &circulation_strength,
                    time,
                    TIME_STEP
                );
            }

            let mut min_update_time = Float::MAX;
            let mut min_correction_time = Float::MAX;

            for _ in 0..5 {
                let mut update_time = 0.0;
                let mut correction_time = 0.0;

                for _ in 0..nr_calls {
                    time += TIME_STEP;

                    let start = Instant::now();

                    correction.update(
                        &line_force_model,
                        &velocity,
                        &circulation_strength,
                        &circulation_strength,
                        time,
                        TIME_STEP
                    );

                    update_time += start.elapsed().as_secs_f64() as Float;

                    let start = Instant::now();

                    let u_i = correction.velocity_correction(
                        &line_force_model,
                        &velocity,
                        &circulation_strength,
                        &circulation_strength,
                        0.0
                    );

                    correction_time += start.elapsed().as_secs_f64() as Float;

                    std::hint::black_box(u_i);
                }

                min_update_time = min_update_time.min(update_time * 1e3 / nr_calls as Float);
                min_correction_time = min_correction_time.min(correction_time * 1e3 / nr_calls as Float);
            }

            println!(
                "  {:<28} include_wing_interactions = {:<5}: {:>8.3} ms update + {:>8.3} ms correction per call",
                name, include_wing_interactions, min_update_time, min_correction_time
            );
        }
    }

    println!();
}
