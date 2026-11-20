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

/// Builds and updates the correction, and returns the velocity correction
fn split_correction(
    builder: LiftingLineCorrectionBuilder,
    line_force_model: &LineForceModel,
    velocity: &[SpatialVector],
    reference_circulation_strength: &[Float],
    projected_circulation_strength: &[Float],
) -> Vec<SpatialVector> {
    let mut correction = builder.build(VISCOUS_CORE_LENGTH_FACTOR, line_force_model).unwrap();

    correction.update(line_force_model, velocity);

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

    correction.update(&line_force_model, &velocity);

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

    correction.update(&line_force_model, &velocity);

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
            let mut min_time = Float::MAX;

            for _ in 0..5 {
                let start = Instant::now();

                for _ in 0..nr_calls {
                    correction.update(&line_force_model, &velocity);

                    let u_i = correction.velocity_correction(
                        &line_force_model,
                        &velocity,
                        &circulation_strength,
                        &circulation_strength,
                        0.0
                    );

                    std::hint::black_box(u_i);
                }

                let time = start.elapsed().as_secs_f64() as Float * 1e3 / nr_calls as Float;

                min_time = min_time.min(time);
            }

            println!(
                "  {:<26} include_wing_interactions = {:<5}: {:>8.3} ms per call",
                name, include_wing_interactions, min_time
            );
        }
    }

    println!();
}
