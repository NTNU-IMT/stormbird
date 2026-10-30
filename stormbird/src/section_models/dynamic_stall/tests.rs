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
        ..Default::default()
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

        let cl_dynamic = total_lift_coefficient(&foil, angle_of_attack, &state);

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
    assert_eq!(default_builder.vortex_time_constant, DynamicStallModelBuilder::default_vortex_time_constant());
    assert_eq!(default_builder.vortex_passage_time, DynamicStallModelBuilder::default_vortex_passage_time());

    let builder: DynamicStallModelBuilder = serde_json::from_str(
        r#"{
            "pressure_time_constant": 2.0,
            "separation_time_constant": 4.0,
            "vortex_time_constant": 5.0,
            "vortex_passage_time": 9.0
        }"#
    ).unwrap();

    assert_eq!(builder.pressure_time_constant, 2.0);
    assert_eq!(builder.separation_time_constant, 4.0);
    assert_eq!(builder.vortex_time_constant, 5.0);
    assert_eq!(builder.vortex_passage_time, 9.0);

    let model = builder.build(5);

    assert_eq!(model.segment_states.len(), 5);
    assert!(!model.is_initialized);
}

#[test]
#[should_panic]
fn negative_time_constants_are_not_allowed() {
    DynamicStallModelBuilder {
        pressure_time_constant: -1.0,
        ..Default::default()
    }.build(1);
}

#[test]
#[should_panic]
fn negative_vortex_time_constants_are_not_allowed() {
    DynamicStallModelBuilder {
        vortex_passage_time: -1.0,
        ..Default::default()
    }.build(1);
}

fn model_with_vortex(vortex_time_constant: Float, vortex_passage_time: Float) -> DynamicStallModel {
    let mut model = DynamicStallModelBuilder {
        vortex_time_constant,
        vortex_passage_time,
        ..Default::default()
    }.build(1);

    model.set_time_step(TIME_STEP);

    model
}

/// The total lift coefficient, computed in the same way as in the line force model: the lift from
/// the section model with the dynamic amount of stall, plus the vortex lift
fn total_lift_coefficient(foil: &Foil, angle_of_attack: Float, state: &DynamicStallSegmentState) -> Float {
    let lift_input = state.lift_input();

    let (cl_pre_stall, cl_post_stall) = SectionModel::SingleElementFoil(foil.clone())
        .lift_coefficients_pre_and_post_stall(angle_of_attack, 0.0, lift_input.amount_of_stall);

    cl_pre_stall + cl_post_stall + lift_input.vortex_lift
}

/// Ramps the angle of attack from 10 to 40 degrees in the given number of steps, and holds it at
/// 40 degrees afterwards. Returns the states for all steps.
fn ramp_through_stall(model: &mut DynamicStallModel, foil: &Foil, nr_ramp_steps: usize, nr_steps: usize) -> Vec<DynamicStallSegmentState> {
    let angle_start = Float::to_radians(10.0);
    let angle_end = Float::to_radians(40.0);

    step(model, foil, angle_start);

    (1..=nr_steps).map(|i| {
        let fraction = (i as Float / nr_ramp_steps as Float).min(1.0);

        step(model, foil, angle_start + fraction * (angle_end - angle_start))
    }).collect()
}

#[test]
fn steady_state_has_no_vortex_lift() {
    let foil = foil();

    for angle_in_degrees in [10.0, 25.0, 90.0] {
        let angle_of_attack = Float::to_radians(angle_in_degrees);

        let mut model = model_with_vortex(6.0, 7.0);

        for _ in 0..10 {
            let state = step(&mut model, &foil, angle_of_attack);

            assert!(state.vortex_lift.abs() < 1e-12);
        }
    }
}

#[test]
fn vortex_is_created_when_the_lagged_angle_exceeds_the_stall_angle() {
    let foil = foil();
    let mut model = model_with_vortex(6.0, 7.0);

    let states = ramp_through_stall(&mut model, &foil, 100, 100);

    for state in states {
        let lagged_angle_above_stall = state.lagged_angle_of_attack().abs() > foil.mean_positive_stall_angle;

        assert_eq!(state.vortex_time > 0.0, lagged_angle_above_stall);

        if state.vortex_time == 0.0 {
            assert!(state.vortex_lift.abs() < 1e-12);
        }
    }
}

#[test]
fn vortex_increases_the_lift_during_a_fast_ramp() {
    let foil = foil();

    let mut model_without_vortex = model_with_vortex(0.0, 7.0);
    let mut model_with_vortex = model_with_vortex(6.0, 7.0);

    let states_without_vortex = ramp_through_stall(&mut model_without_vortex, &foil, 40, 40);
    let states_with_vortex = ramp_through_stall(&mut model_with_vortex, &foil, 40, 40);

    let mut max_lift_without_vortex: Float = 0.0;
    let mut max_lift_with_vortex: Float = 0.0;

    for (state_without, state_with) in states_without_vortex.iter().zip(states_with_vortex.iter()) {
        assert!(state_without.vortex_lift.abs() < 1e-12);
        assert!(state_with.vortex_lift >= -1e-12);

        // The vortex does not affect the separation model
        assert_eq!(state_without.amount_of_stall, state_with.amount_of_stall);

        max_lift_without_vortex = max_lift_without_vortex.max(
            total_lift_coefficient(&foil, state_without.angle_of_attack, state_without)
        );
        max_lift_with_vortex = max_lift_with_vortex.max(
            total_lift_coefficient(&foil, state_with.angle_of_attack, state_with)
        );
    }

    assert!(
        max_lift_with_vortex > max_lift_without_vortex,
        "The vortex does not increase the max lift. Without = {}, with = {}",
        max_lift_without_vortex, max_lift_with_vortex
    );
}

#[test]
fn vortex_lift_only_decays_after_the_vortex_has_passed() {
    let foil = foil();

    let vortex_time_constant = 6.0;
    let vortex_passage_time = 7.0;

    let mut model = model_with_vortex(vortex_time_constant, vortex_passage_time);

    let delta_s = delta_s(&model);

    let states = ramp_through_stall(&mut model, &foil, 40, 2000);

    let decay = (-delta_s / vortex_time_constant).exp();

    let mut nr_steps_after_passage = 0;

    for window in states.windows(2) {
        let (previous, current) = (&window[0], &window[1]);

        if current.vortex_time > vortex_passage_time {
            assert!((current.vortex_lift - previous.vortex_lift * decay).abs() < 1e-12);

            nr_steps_after_passage += 1;
        }
    }

    assert!(nr_steps_after_passage > 0);

    let last_state = states.last().unwrap();

    assert!(last_state.vortex_lift.abs() < 1e-6);

    // The static lift is recovered in the end
    let cl_dynamic = total_lift_coefficient(&foil, last_state.angle_of_attack, last_state);
    let cl_static = foil.lift_coefficient(last_state.angle_of_attack);

    assert!((cl_dynamic - cl_static).abs() < 1e-6);
}

#[test]
fn zero_vortex_passage_time_turns_the_vortex_off() {
    let foil = foil();
    let mut model = model_with_vortex(6.0, 0.0);

    for state in ramp_through_stall(&mut model, &foil, 40, 200) {
        assert!(state.vortex_lift.abs() < 1e-12);
    }
}

#[test]
fn deficiency_function_gives_the_discrete_step_response() {
    let delta_s = 0.1;
    let time_constant = 2.0;

    // A unit step in the input at the first time step, and a constant input afterwards
    let mut deficiency = DynamicStallModel::deficiency_function(0.0, 1.0, delta_s, time_constant);

    for i in 1..50 {
        deficiency = DynamicStallModel::deficiency_function(deficiency, 0.0, delta_s, time_constant);

        let expected = (-delta_s / (2.0 * time_constant)).exp() *
            (-(i as Float) * delta_s / time_constant).exp();

        assert!(
            (deficiency - expected).abs() < 1e-12,
            "Wrong deficiency after {} steps: {} (expected {})", i, deficiency, expected
        );
    }
}

#[test]
fn deficiency_function_is_zero_without_time_constant() {
    for time_constant in [0.0, -1.0] {
        assert_eq!(DynamicStallModel::deficiency_function(0.5, 1.0, 0.1, time_constant), 0.0);
    }
}
