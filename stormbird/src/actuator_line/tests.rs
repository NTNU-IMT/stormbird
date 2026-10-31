// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Tests of the dynamic stall model in actuator line simulations

use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

use crate::line_force_model::builder::LineForceModelBuilder;
use crate::line_force_model::builder::single_wing::WingBuilder;
use crate::section_models::SectionModel;
use crate::section_models::foil::Foil;
use crate::section_models::dynamic_stall::DynamicStallModelBuilder;

use super::ActuatorLine;
use super::builder::ActuatorLineBuilder;
use super::solver::SolverSettings;

const VELOCITY: Float = 10.0;
const TIME_STEP: Float = 0.01;
const MAX_ANGLE_DEG: Float = 35.0;
const NR_TIME_STEPS: usize = 40;

/// An actuator line model of a single wing, without damping of the circulation, so that the result
/// in each call to `do_step` only depends on the velocity and the state of the dynamic stall model
fn actuator_line(dynamic_stall: Option<DynamicStallModelBuilder>) -> ActuatorLine {
    let mut line_force_model = LineForceModelBuilder::new(10);

    line_force_model.add_wing(WingBuilder {
        section_points: vec![
            SpatialVector::from([0.0, 0.0, 0.0]),
            SpatialVector::from([0.0, 0.0, 5.0]),
        ],
        chord_vectors: vec![
            SpatialVector::from([1.0, 0.0, 0.0]),
            SpatialVector::from([1.0, 0.0, 0.0]),
        ],
        section_model: SectionModel::SingleElementFoil(Foil::default()),
        dynamic_stall,
        ..Default::default()
    });

    ActuatorLineBuilder {
        solver_settings: SolverSettings { damping_factor: 1.0 },
        ..ActuatorLineBuilder::new(line_force_model)
    }.build()
}

/// Ramps the wing angle linearly from zero to the max angle, with a constant velocity at the
/// control points, and calls `do_step` the given number of times in each time step, as in a CFD
/// solver with outer iterations. Returns the lift and the amount of stall on the first line element
/// at each time step.
fn ramp(actuator_line: &mut ActuatorLine, nr_iterations_per_step: usize) -> (Vec<Float>, Vec<Float>) {
    let max_angle = Float::to_radians(MAX_ANGLE_DEG);

    let mut lift = Vec::with_capacity(NR_TIME_STEPS);
    let mut amount_of_stall = Vec::with_capacity(NR_TIME_STEPS);

    for i in 0..NR_TIME_STEPS {
        let time = i as Float * TIME_STEP;

        let wing_angle = max_angle * (i as Float / (NR_TIME_STEPS - 1) as Float);

        actuator_line.line_force_model.set_local_wing_angles(&[wing_angle]);

        actuator_line.ctrl_points_velocity = vec![
            SpatialVector::from([VELOCITY, 0.0, 0.0]);
            actuator_line.line_force_model.nr_span_lines()
        ];

        for _ in 0..nr_iterations_per_step {
            actuator_line.do_step(time, TIME_STEP);
        }

        let result = actuator_line.simulation_result.as_ref().unwrap();

        lift.push(result.integrated_forces_sum()[1].abs());
        amount_of_stall.push(result.force_input.amount_of_stall[0]);
    }

    (lift, amount_of_stall)
}

fn assert_all_close(a: &[Float], b: &[Float]) {
    for (a, b) in a.iter().zip(b.iter()) {
        assert!(
            (a - b).abs() <= 1e-9 * a.abs().max(1.0),
            "Different values: {} != {}", a, b
        );
    }
}

#[test]
fn outer_iterations_do_not_change_the_dynamic_stall_state() {
    let (lift_single, stall_single) = ramp(
        &mut actuator_line(Some(DynamicStallModelBuilder::default())), 1
    );
    let (lift_multiple, stall_multiple) = ramp(
        &mut actuator_line(Some(DynamicStallModelBuilder::default())), 4
    );

    assert_all_close(&lift_single, &lift_multiple);
    assert_all_close(&stall_single, &stall_multiple);
}

#[test]
fn zero_time_constants_give_the_same_result_as_without_dynamic_stall() {
    let (lift_static, stall_static) = ramp(&mut actuator_line(None), 1);
    let (lift_dynamic, stall_dynamic) = ramp(
        &mut actuator_line(Some(DynamicStallModelBuilder {
            pressure_time_constant: Some(0.0),
            separation_time_constant: Some(0.0),
            vortex_time_constant: Some(0.0),
            ..Default::default()
        })),
        1
    );

    assert_all_close(&lift_static, &lift_dynamic);
    assert_all_close(&stall_static, &stall_dynamic);
}

#[test]
fn dynamic_stall_increases_the_max_lift_during_a_fast_ramp() {
    let (lift_static, _) = ramp(&mut actuator_line(None), 1);
    let (lift_dynamic, _) = ramp(&mut actuator_line(Some(DynamicStallModelBuilder::default())), 1);

    let max_lift_static = lift_static.iter().cloned().fold(0.0, Float::max);
    let max_lift_dynamic = lift_dynamic.iter().cloned().fold(0.0, Float::max);

    assert!(
        max_lift_dynamic > 1.05 * max_lift_static,
        "The dynamic stall model does not increase the max lift. Static = {}, dynamic = {}",
        max_lift_static, max_lift_dynamic
    );
}

#[test]
fn amount_of_stall_in_the_result_is_the_one_used_for_the_forces() {
    let mut actuator_line_static = actuator_line(None);
    let mut actuator_line_dynamic = actuator_line(Some(DynamicStallModelBuilder::default()));

    let (_, stall_static) = ramp(&mut actuator_line_static, 1);
    let (_, stall_dynamic) = ramp(&mut actuator_line_dynamic, 1);

    // Without dynamic stall, the amount of stall is the static value from the foil model
    let result = actuator_line_static.simulation_result.as_ref().unwrap();

    let foil = Foil::default();

    for (angle_of_attack, amount_of_stall) in result.force_input.angles_of_attack.iter()
        .zip(result.force_input.amount_of_stall.iter())
    {
        assert_eq!(*amount_of_stall, foil.amount_of_stall(*angle_of_attack));
    }

    // The foil is stalled at the end of the ramp, but the dynamic amount of stall is lagging
    let last_index = NR_TIME_STEPS - 1;

    assert!(stall_static[last_index] > 0.5);
    assert!(
        stall_dynamic[last_index] < stall_static[last_index],
        "The dynamic amount of stall is not lagging. Static = {}, dynamic = {}",
        stall_static[last_index], stall_dynamic[last_index]
    );
}
