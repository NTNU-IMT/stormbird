// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Tests of the dynamic stall model in lifting line simulations

use stormath::type_aliases::Float;

use crate::lifting_line::prelude::*;
use crate::lifting_line::simulation::Simulation;
use crate::lifting_line::simulation_builder::{
    SimulationBuilder,
    SimulationSettings,
    QuasiSteadySettings,
    DynamicSettings,
};
use crate::section_models::dynamic_stall::DynamicStallModelBuilder;

const VELOCITY: Float = 10.0;
const TIME_STEP: Float = 0.01;

fn line_force_model_builder(dynamic_stall: Option<DynamicStallModelBuilder>) -> LineForceModelBuilder {
    let mut builder = LineForceModelBuilder::new(10);

    builder.add_wing(WingBuilder {
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

    builder
}

fn dynamic_simulation(dynamic_stall: Option<DynamicStallModelBuilder>) -> Simulation {
    SimulationBuilder::new(
        line_force_model_builder(dynamic_stall),
        SimulationSettings::Dynamic(DynamicSettings::default()),
    ).build()
}

/// Runs a simulation where the wing angle is ramped linearly from zero to the max angle, and returns
/// the magnitude of the lift force at each time step.
fn lift_during_ramp(simulation: &mut Simulation, max_angle: Float, nr_time_steps: usize) -> Vec<Float> {
    let freestream_velocity = SpatialVector::from([VELOCITY, 0.0, 0.0]);

    let mut lift = Vec::with_capacity(nr_time_steps);

    for i in 0..nr_time_steps {
        let time = i as Float * TIME_STEP;

        let wing_angle = max_angle * (i as Float / (nr_time_steps - 1) as Float);

        simulation.line_force_model.set_local_wing_angles(&[wing_angle]);

        let nr_points = simulation.get_freestream_velocity_points().len();

        let result = simulation.do_step(time, TIME_STEP, &vec![freestream_velocity; nr_points]);

        lift.push(result.integrated_forces_sum()[1].abs());
    }

    lift
}

#[test]
fn dynamic_stall_is_not_allowed_in_quasi_steady_simulations() {
    let builder = SimulationBuilder::new(
        line_force_model_builder(Some(DynamicStallModelBuilder::default())),
        SimulationSettings::QuasiSteady(QuasiSteadySettings::default()),
    );

    assert!(builder.check_settings().is_err());
}

#[test]
fn zero_time_constants_give_the_same_result_as_without_dynamic_stall() {
    let max_angle = Float::to_radians(35.0);
    let nr_time_steps = 40;

    let mut simulation_static = dynamic_simulation(None);
    let mut simulation_dynamic = dynamic_simulation(Some(DynamicStallModelBuilder {
        pressure_time_constant: 0.0,
        separation_time_constant: 0.0,
    }));

    let lift_static = lift_during_ramp(&mut simulation_static, max_angle, nr_time_steps);
    let lift_dynamic = lift_during_ramp(&mut simulation_dynamic, max_angle, nr_time_steps);

    for (static_value, dynamic_value) in lift_static.iter().zip(lift_dynamic.iter()) {
        assert!(
            (static_value - dynamic_value).abs() <= 1e-9 * static_value.abs().max(1.0),
            "Different lift with zero time constants: static = {}, dynamic = {}",
            static_value, dynamic_value
        );
    }
}

#[test]
fn dynamic_stall_increases_the_max_lift_during_a_fast_ramp() {
    let max_angle = Float::to_radians(35.0);
    let nr_time_steps = 40;

    let mut simulation_static = dynamic_simulation(None);
    let mut simulation_dynamic = dynamic_simulation(Some(DynamicStallModelBuilder::default()));

    let lift_static = lift_during_ramp(&mut simulation_static, max_angle, nr_time_steps);
    let lift_dynamic = lift_during_ramp(&mut simulation_dynamic, max_angle, nr_time_steps);

    let max_lift_static = lift_static.iter().cloned().fold(0.0, Float::max);
    let max_lift_dynamic = lift_dynamic.iter().cloned().fold(0.0, Float::max);

    dbg!(max_lift_static, max_lift_dynamic);

    assert!(
        max_lift_dynamic > 1.05 * max_lift_static,
        "The dynamic stall model does not increase the max lift. Static = {}, dynamic = {}",
        max_lift_static, max_lift_dynamic
    );
}
