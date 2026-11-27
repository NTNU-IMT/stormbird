// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use crate::lifting_line::prelude::*;
use crate::lifting_line::simulation_builder::{
    SimulationBuilder,
    SimulationSettings,
    DynamicSettings,
    QuasiSteadySettings,
};
use crate::lifting_line::solvers::{
    Solver,
    QuasiSteadySolverBuilder,
    iterative::{Iterative, QuasiSteadyIterativeBuilder},
    velocity_corrections::InducedVelocitySettings,
};

use stormath::type_aliases::Float;

fn neglect_self_induced_velocities() -> InducedVelocitySettings {
    InducedVelocitySettings {
        neglect_self_induced_velocities: true,
        ..Default::default()
    }
}

/// Simulates two rotor sails that are placed far apart, so that the velocities induced by the
/// other sail should be very close to zero, and returns the relative error in the lift coefficient
/// compared to the two-dimensional value.
fn relative_lift_error_for_two_sails_far_apart(
    simulation_settings: SimulationSettings,
    nr_time_steps: usize,
    time_step: Float,
) -> Float {
    let nr_strips = 10;
    let diameter  = 1.0;
    let span      = 5.0;
    let velocity_mag = 1.2;

    let spin_ratio = 2.0;

    let revolutions_per_second = RotatingCylinder::revolutions_per_second_from_spin_ratio(
        spin_ratio, diameter, velocity_mag
    );

    let section_model = RotatingCylinder::default();

    let mut line_force_model_builder = LineForceModelBuilder::new(nr_strips);

    let chord_vector = SpatialVector::from([diameter, 0.0, 0.0]);

    let wing_x_positions = [-span,
        1.0 * span];

    let wing_y_positions = [-10.0 * span,
        10.0 * span];

    for (x_pos, y_pos) in wing_x_positions.iter().zip(wing_y_positions.iter()) {
        let wing_builder = WingBuilder {
            section_points: vec![
                SpatialVector::from([*x_pos, *y_pos, 0.0]),
                SpatialVector::from([*x_pos, *y_pos, span]),
            ],
            chord_vectors: vec![
                chord_vector,
                chord_vector,
            ],
            section_model: SectionModel::RotatingCylinder(section_model.clone()),
            non_zero_circulation_at_ends: [false, false],
            ..Default::default()
        };

        line_force_model_builder.add_wing(wing_builder);
    }

    let velocity = SpatialVector::from([velocity_mag, 0.0, 0.0]);

    let mut sim = SimulationBuilder::new(
        line_force_model_builder,
        simulation_settings
    ).build().unwrap();

    sim.line_force_model.set_internal_state(
        &vec![revolutions_per_second; wing_x_positions.len()]
    );

    let force_factor = sim.line_force_model.total_force_factor(velocity.length());

    let freestream_velocity_points = sim.get_freestream_velocity_points();
    let input_freestream_velocity = vec![velocity; freestream_velocity_points.len()];

    let mut result = SimulationResult::default();

    for i in 0..nr_time_steps {
        let time = (i as Float) * time_step;

        result = sim.do_step(time, time_step, &input_freestream_velocity);
    }

    let cd = result.integrated_forces_sum()[0] / force_factor;
    let cl = result.integrated_forces_sum()[1] / force_factor;

    dbg!(&result.force_input.velocity);

    dbg!(cd, cl);

    //let cd_data = 1.497;  TODO make the drag comparison work again
    let cl_data = section_model.lift_coefficient(spin_ratio);

    let cl_error = cl - cl_data;

    dbg!(cl_error.abs() / cl_data);

    cl_error.abs() / cl_data
}

#[test]
/// This tests whether the lift and drag coefficients are correct for a steady simulation of two
/// sails when the "neglect_self_induced_velocity" flag is set to true. The sails are placed far
/// apart so that the induced velocities should be very close to zero.
///
/// The test was created as a result of a bug where the lift and drag coefficients were not correct
/// with multiple sails, but correct with only one sail.
fn no_self_induced_velocity() {
    let settings = DynamicSettings{
        solver: Solver::Iterative(Iterative {
            induced_velocities: neglect_self_induced_velocities(),
            ..Default::default()
        }),
        ..Default::default()
    };

    let relative_error = relative_lift_error_for_two_sails_far_apart(
        SimulationSettings::Dynamic(settings),
        100,
        0.5
    );

    assert!(relative_error < 0.0001);
}

#[test]
/// Same as [`no_self_induced_velocity`], but with a quasi-steady simulation, which is supported now
/// that the setting is part of the solver rather than the dynamic wake.
fn no_self_induced_velocity_quasi_steady() {
    let settings = QuasiSteadySettings {
        solver: QuasiSteadySolverBuilder::Iterative(QuasiSteadyIterativeBuilder {
            induced_velocities: neglect_self_induced_velocities(),
            ..Default::default()
        }),
        ..Default::default()
    };

    let relative_error = relative_lift_error_for_two_sails_far_apart(
        SimulationSettings::QuasiSteady(settings),
        1,
        1.0
    );

    assert!(relative_error < 0.0001);
}
