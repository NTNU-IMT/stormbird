// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use super::*;

const VELOCITY: Float = 10.0;
const CHORD_LENGTH: Float = 2.0;
const TIME_STEP: Float = 0.01;

fn foil() -> Foil {
    Foil::default()
}

fn model(pressure_time_constant: Float, separation_time_constant: Float) -> DynamicStallModel {
    let mut model = DynamicStallModelBuilder {
        pressure_time_constant,
        separation_time_constant,
    }.build(1);

    model.set_time_step(TIME_STEP);

    model
}

/// Evaluates the model at the given angle of attack and commits the result, as done at the end of
/// each time step in a simulation.
fn step(model: &mut DynamicStallModel, foil: &Foil, angle_of_attack: Float) -> DynamicStallSegmentState {
    let state = model.evaluate(0, foil, angle_of_attack, VELOCITY, CHORD_LENGTH);

    model.commit(vec![state]);

    state
}

fn delta_s(model: &DynamicStallModel) -> Float {
    model.non_dimensional_time_step(VELOCITY, CHORD_LENGTH)
}

#[test]
fn uninitialized_model_gives_static_values() {
    let foil = foil();
    let model = model(1.7, 3.0);

    for angle_in_degrees in [-170.0, -25.0, -10.0, 0.0, 10.0, 19.0, 25.0, 90.0, 170.0] {
        let angle_of_attack = Float::to_radians(angle_in_degrees);

        let state = model.evaluate(0, &foil, angle_of_attack, VELOCITY, CHORD_LENGTH);

        assert_eq!(state.amount_of_stall, foil.amount_of_stall(angle_of_attack));

        let cl_dynamic =
            DynamicStallModel::lift_coefficient_pre_stall_with_stall_drop_off(&foil, angle_of_attack, &state) +
            DynamicStallModel::lift_coefficient_post_stall_with_stall_weight(&foil, angle_of_attack, &state);

        let cl_static = foil.lift_coefficient(angle_of_attack);

        assert!(
            (cl_dynamic - cl_static).abs() < 1e-12,
            "Static lift not recovered at {} degrees: dynamic = {}, static = {}",
            angle_in_degrees, cl_dynamic, cl_static
        );
    }
}

#[test]
fn constant_angle_of_attack_gives_static_values() {
    let foil = foil();

    for angle_in_degrees in [-30.0, 0.0, 18.0, 22.0, 60.0] {
        let angle_of_attack = Float::to_radians(angle_in_degrees);

        let mut model = model(1.7, 3.0);

        for _ in 0..10 {
            let state = step(&mut model, &foil, angle_of_attack);

            assert!(
                (state.amount_of_stall - foil.amount_of_stall(angle_of_attack)).abs() < 1e-12
            );
            assert!(state.pressure_deficiency.abs() < 1e-12);
            assert!(state.separation_deficiency.abs() < 1e-12);
        }
    }
}

#[test]
fn pressure_lag_follows_the_deficiency_function() {
    let foil = foil();

    let time_constant = 1.7;
    let mut model = model(time_constant, 0.0);

    let angle_start = Float::to_radians(5.0);
    let angle_increment = Float::to_radians(10.0);

    step(&mut model, &foil, angle_start);

    let delta_s = delta_s(&model);

    // The first step after a step change in the angle of attack
    let state = step(&mut model, &foil, angle_start + angle_increment);

    let expected_deficiency = angle_increment * (-delta_s / (2.0 * time_constant)).exp();

    assert!((state.pressure_deficiency - expected_deficiency).abs() < 1e-12);

    // The following steps, where the angle of attack is constant
    let mut expected_deficiency = expected_deficiency;

    for _ in 0..5 {
        let state = step(&mut model, &foil, angle_start + angle_increment);

        expected_deficiency *= (-delta_s / time_constant).exp();

        assert!((state.pressure_deficiency - expected_deficiency).abs() < 1e-12);
    }
}

#[test]
fn zero_time_constants_give_static_values() {
    let foil = foil();
    let mut model = model(0.0, 0.0);

    step(&mut model, &foil, 0.0);

    for angle_in_degrees in [10.0, 20.0, 30.0, 15.0, -25.0] {
        let angle_of_attack = Float::to_radians(angle_in_degrees);

        let state = step(&mut model, &foil, angle_of_attack);

        assert!((state.amount_of_stall - foil.amount_of_stall(angle_of_attack)).abs() < 1e-12);
    }
}

#[test]
fn stall_is_delayed_when_the_angle_of_attack_increases() {
    let foil = foil();
    let mut model = model(1.7, 3.0);

    let angle_before_stall = Float::to_radians(10.0);
    let angle_after_stall = Float::to_radians(30.0);

    step(&mut model, &foil, angle_before_stall);

    let static_amount_of_stall = foil.amount_of_stall(angle_after_stall);

    let mut previous_amount_of_stall = foil.amount_of_stall(angle_before_stall);

    for _ in 0..2000 {
        let state = step(&mut model, &foil, angle_after_stall);

        assert!(state.amount_of_stall < static_amount_of_stall + 1e-12);
        assert!(state.amount_of_stall >= previous_amount_of_stall - 1e-12);

        previous_amount_of_stall = state.amount_of_stall;
    }

    assert!((previous_amount_of_stall - static_amount_of_stall).abs() < 1e-6);
}

#[test]
fn reattachment_is_delayed_when_the_angle_of_attack_decreases() {
    let foil = foil();
    let mut model = model(1.7, 3.0);

    let angle_after_stall = Float::to_radians(30.0);
    let angle_before_stall = Float::to_radians(10.0);

    step(&mut model, &foil, angle_after_stall);

    let static_amount_of_stall = foil.amount_of_stall(angle_before_stall);

    let first_state = step(&mut model, &foil, angle_before_stall);

    assert!(first_state.amount_of_stall > static_amount_of_stall + 0.1);

    let mut last_amount_of_stall = first_state.amount_of_stall;

    for _ in 0..2000 {
        last_amount_of_stall = step(&mut model, &foil, angle_before_stall).amount_of_stall;
    }

    assert!((last_amount_of_stall - static_amount_of_stall).abs() < 1e-6);
}

#[test]
fn angle_differences_are_wrapped() {
    let foil = foil();
    let mut model = model(1.7, 3.0);

    step(&mut model, &foil, Float::to_radians(179.0));

    // Going from 179 to -179 degrees is an increase of 2 degrees, through 180 degrees
    let state = step(&mut model, &foil, Float::to_radians(-179.0));

    assert!(state.pressure_deficiency.abs() < Float::to_radians(2.0) + 1e-12);
    assert!(state.pressure_deficiency > 0.0);
    assert!((state.lagged_angle_of_attack().abs() - Float::to_radians(179.0)).abs() < Float::to_radians(1.0));
}

#[test]
fn reset_gives_static_values() {
    let foil = foil();
    let mut model = model(1.7, 3.0);

    step(&mut model, &foil, Float::to_radians(10.0));

    model.reset();

    let angle_of_attack = Float::to_radians(30.0);

    let state = model.evaluate(0, &foil, angle_of_attack, VELOCITY, CHORD_LENGTH);

    assert_eq!(state.amount_of_stall, foil.amount_of_stall(angle_of_attack));
}

#[test]
fn only_foil_section_models_are_supported() {
    assert!(DynamicStallModel::is_supported(&SectionModel::SingleElementFoil(Foil::default())));
    assert!(!DynamicStallModel::is_supported(&SectionModel::RotatingCylinder(Default::default())));
    assert!(!DynamicStallModel::is_supported(&SectionModel::EffectiveWindSensor));
}

#[test]
fn builder_from_json() {
    let default_builder: DynamicStallModelBuilder = serde_json::from_str("{}").unwrap();

    assert_eq!(default_builder.pressure_time_constant, DynamicStallModelBuilder::default_pressure_time_constant());
    assert_eq!(default_builder.separation_time_constant, DynamicStallModelBuilder::default_separation_time_constant());

    let builder: DynamicStallModelBuilder = serde_json::from_str(
        r#"{"pressure_time_constant": 2.0, "separation_time_constant": 4.0}"#
    ).unwrap();

    assert_eq!(builder.pressure_time_constant, 2.0);
    assert_eq!(builder.separation_time_constant, 4.0);

    let model = builder.build(5);

    assert_eq!(model.segment_states.len(), 5);
    assert!(!model.is_initialized);
}

#[test]
#[should_panic]
fn negative_time_constants_are_not_allowed() {
    DynamicStallModelBuilder {
        pressure_time_constant: -1.0,
        separation_time_constant: 3.0,
    }.build(1);
}
