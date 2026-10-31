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
/// the integrated force at each time step.
fn forces_during_ramp(simulation: &mut Simulation, max_angle: Float, nr_time_steps: usize) -> Vec<SpatialVector> {
    let freestream_velocity = SpatialVector::from([VELOCITY, 0.0, 0.0]);

    let mut forces = Vec::with_capacity(nr_time_steps);

    for i in 0..nr_time_steps {
        let time = i as Float * TIME_STEP;

        let wing_angle = max_angle * (i as Float / (nr_time_steps - 1) as Float);

        simulation.line_force_model.set_local_wing_angles(&[wing_angle]);

        let nr_points = simulation.get_freestream_velocity_points().len();

        let result = simulation.do_step(time, TIME_STEP, &vec![freestream_velocity; nr_points]);

        forces.push(result.integrated_forces_sum());
    }

    forces
}

/// The magnitude of the lift force at each time step, from [`forces_during_ramp`]
fn lift_during_ramp(simulation: &mut Simulation, max_angle: Float, nr_time_steps: usize) -> Vec<Float> {
    forces_during_ramp(simulation, max_angle, nr_time_steps).iter()
        .map(|force| force[1].abs())
        .collect()
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
fn negative_time_constants_give_an_error() {
    let builder = SimulationBuilder::new(
        line_force_model_builder(Some(DynamicStallModelBuilder {
            separation_time_constant: Some(-1.0),
            ..Default::default()
        })),
        SimulationSettings::Dynamic(DynamicSettings::default()),
    );

    assert!(builder.check_settings().is_err());
    assert!(Simulation::new_from_string(&serde_json::to_string(&builder).unwrap()).is_err());
}

#[test]
fn foil_parameters_for_a_rotor_give_an_error() {
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
        section_model: SectionModel::RotatingCylinder(RotatingCylinder::default()),
        dynamic_stall: Some(DynamicStallModelBuilder {
            pressure_time_constant: Some(1.0),
            ..Default::default()
        }),
        ..Default::default()
    });

    let builder = SimulationBuilder::new(
        line_force_model,
        SimulationSettings::Dynamic(DynamicSettings::default()),
    );

    assert!(builder.check_settings().is_err());
}

#[test]
fn zero_time_constants_give_the_same_result_as_without_dynamic_stall() {
    let max_angle = Float::to_radians(35.0);
    let nr_time_steps = 40;

    let mut simulation_static = dynamic_simulation(None);
    let mut simulation_dynamic = dynamic_simulation(Some(DynamicStallModelBuilder {
        pressure_time_constant: Some(0.0),
        separation_time_constant: Some(0.0),
        vortex_time_constant: Some(0.0),
        ..Default::default()
    }));

    let forces_static = forces_during_ramp(&mut simulation_static, max_angle, nr_time_steps);
    let forces_dynamic = forces_during_ramp(&mut simulation_dynamic, max_angle, nr_time_steps);

    // Both the lift and the drag should be the same
    for (static_force, dynamic_force) in forces_static.iter().zip(forces_dynamic.iter()) {
        for i in 0..3 {
            assert!(
                (static_force[i] - dynamic_force[i]).abs() <= 1e-9 * static_force[i].abs().max(1.0),
                "Different forces with zero time constants: static = {:?}, dynamic = {:?}",
                static_force, dynamic_force
            );
        }
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

fn rotor_simulation(dynamic_stall: Option<DynamicStallModelBuilder>) -> Simulation {
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
        section_model: SectionModel::RotatingCylinder(RotatingCylinder::default()),
        dynamic_stall,
        ..Default::default()
    });

    SimulationBuilder::new(
        builder,
        SimulationSettings::Dynamic(DynamicSettings::default()),
    ).build()
}

/// Runs a rotor simulation where the revolutions per second change in a step after the first time
/// step, and returns the integrated force at each time step.
fn rotor_forces_during_step(simulation: &mut Simulation, nr_time_steps: usize) -> Vec<SpatialVector> {
    let freestream_velocity = SpatialVector::from([VELOCITY, 0.0, 0.0]);

    (0..nr_time_steps).map(|i| {
        let revolutions_per_second = if i == 0 { 0.5 } else { 2.0 };

        simulation.line_force_model.set_internal_state(&[revolutions_per_second]);

        let nr_points = simulation.get_freestream_velocity_points().len();

        simulation.do_step(i as Float * TIME_STEP, TIME_STEP, &vec![freestream_velocity; nr_points])
            .integrated_forces_sum()
    }).collect()
}

#[test]
fn rotor_without_internal_state_lag_gives_the_same_result_as_without_dynamic_stall() {
    let nr_time_steps = 20;

    let forces_static = rotor_forces_during_step(&mut rotor_simulation(None), nr_time_steps);
    let forces_dynamic = rotor_forces_during_step(
        &mut rotor_simulation(Some(DynamicStallModelBuilder::default())), nr_time_steps
    );

    for (static_force, dynamic_force) in forces_static.iter().zip(forces_dynamic.iter()) {
        for i in 0..3 {
            assert!(
                (static_force[i] - dynamic_force[i]).abs() <= 1e-9 * static_force[i].abs().max(1.0),
                "Different forces without internal state lag: static = {:?}, dynamic = {:?}",
                static_force, dynamic_force
            );
        }
    }
}

#[test]
fn internal_state_lag_delays_the_rotor_lift() {
    let nr_time_steps = 400;

    let forces_static = rotor_forces_during_step(&mut rotor_simulation(None), nr_time_steps);
    let forces_dynamic = rotor_forces_during_step(
        &mut rotor_simulation(Some(DynamicStallModelBuilder {
            internal_state_time_constant: 5.0,
            ..Default::default()
        })),
        nr_time_steps
    );

    // Right after the step, the lift is lower with the lag
    assert!(
        forces_dynamic[1][1].abs() < 0.9 * forces_static[1][1].abs(),
        "The internal state lag does not delay the lift. Static = {:?}, dynamic = {:?}",
        forces_static[1], forces_dynamic[1]
    );

    // After a long time, the forces are the same
    let last_static = forces_static.last().unwrap();
    let last_dynamic = forces_dynamic.last().unwrap();

    for i in 0..3 {
        assert!(
            (last_static[i] - last_dynamic[i]).abs() <= 1e-3 * last_static.length(),
            "The static forces are not recovered. Static = {:?}, dynamic = {:?}",
            last_static, last_dynamic
        );
    }
}
