// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Timing benchmark for the calculation of induced velocities in lifting line simulations.
//!
//! The benchmark is ignored by default, as it is only useful in release mode. Run it with:
//!
//! ```text
//! cargo test --release induced_velocity_benchmark -- --ignored --nocapture
//! cargo test --release --features parallel induced_velocity_benchmark -- --ignored --nocapture
//! ```

use crate::lifting_line::prelude::*;

use crate::lifting_line::solvers::{
    QuasiSteadySolverBuilder,
    iterative::QuasiSteadyIterativeBuilder,
};
use crate::lifting_line::simulation::Simulation;
use crate::lifting_line::simulation_builder::{
    SimulationBuilder,
    SimulationSettings,
    QuasiSteadySettings,
    DynamicSettings,
};

use stormath::type_aliases::Float;

use std::time::Instant;

const NR_WINGS: usize = 3;
const NR_STRIPS: usize = 40;

/// Three wing sails after each other, with enough distance between them to be a realistic case
fn line_force_model_builder() -> LineForceModelBuilder {
    let x_positions = [-5.0, 0.0, 5.0];
    let y_positions = [5.0, -5.0, 5.0];

    let span = 10.0;

    let mut line_force_model_builder = LineForceModelBuilder::new(NR_STRIPS);

    for i in 0..NR_WINGS {
        let wing_builder = WingBuilder {
            section_points: vec![
                SpatialVector::from([x_positions[i], y_positions[i], 10.0]),
                SpatialVector::from([x_positions[i], y_positions[i], 10.0 + span]),
            ],
            chord_vectors: vec![SpatialVector::from([1.0, 0.0, 0.0]); 2],
            section_model: SectionModel::SingleElementFoil(Foil {
                cl_zero_angle: 0.5,
                ..Default::default()
            }),
            non_zero_circulation_at_ends: [false, false],
            ..Default::default()
        };

        line_force_model_builder.add_wing(wing_builder);
    }

    line_force_model_builder
}

/// Runs the function the given number of times, and returns the minimum time per call in
/// microseconds. The minimum is used, as it is the least sensitive to noise from other processes.
fn min_time_per_call_us<F: FnMut()>(nr_repetitions: usize, nr_calls: usize, mut f: F) -> Float {
    let mut min_time = Float::MAX;

    for _ in 0..nr_repetitions {
        let start = Instant::now();

        for _ in 0..nr_calls {
            f();
        }

        let time = start.elapsed().as_secs_f64() as Float * 1e6 / nr_calls as Float;

        min_time = min_time.min(time);
    }

    min_time
}

fn run_time_steps(sim: &mut Simulation, nr_time_steps: usize, time_step: Float) {
    let freestream_velocity = SpatialVector::from([8.0, 2.0, 0.0]);

    let nr_points = sim.get_freestream_velocity_points().len();
    let input_freestream_velocity = vec![freestream_velocity; nr_points];

    for i in 0..nr_time_steps {
        sim.do_step(i as Float * time_step, time_step, &input_freestream_velocity);
    }
}

#[test]
#[ignore]
/// Times the parts of the lifting line simulation that depend on the induced velocity calculations
fn induced_velocity_benchmark() {
    let time_step = 0.05;

    // --------------------------- Dynamic simulation ----------------------------------------------
    let mut sim = SimulationBuilder::new(
        line_force_model_builder(),
        SimulationSettings::Dynamic(DynamicSettings::default())
    ).build().unwrap();

    // Warm up, so that the wake has non-zero strength everywhere
    run_time_steps(&mut sim, 120, time_step);

    let ctrl_points = sim.line_force_model.ctrl_points_global.clone();
    let strengths = sim.previous_circulation_strength.clone();

    let WakeData::Dynamic(wake) = sim.wake_data.clone() else {
        panic!("Expected a dynamic wake");
    };

    let mut frozen_wake = sim.frozen_wake.clone();

    let t_matvec = min_time_per_call_us(5, 20_000, || {
        frozen_wake.update_induced_velocities_at_control_points(&strengths);
    });

    let t_fixed = min_time_per_call_us(5, 20, || {
        frozen_wake.update_fixed_velocities(&ctrl_points, &wake);
    });

    let t_variable = min_time_per_call_us(5, 20, || {
        frozen_wake.update_variable_velocity_factors(&ctrl_points, &wake);
    });

    let t_dynamic_step = min_time_per_call_us(5, 1, || {
        run_time_steps(&mut sim, 10, time_step);
    }) / 10.0;

    // --------------------------- Quasi-steady simulation -----------------------------------------
    let mut sim_qs = SimulationBuilder::new(
        line_force_model_builder(),
        SimulationSettings::QuasiSteady(QuasiSteadySettings {
            solver: QuasiSteadySolverBuilder::Iterative(QuasiSteadyIterativeBuilder::default()),
            ..Default::default()
        })
    ).build().unwrap();

    run_time_steps(&mut sim_qs, 2, 1.0);

    let t_quasi_steady_step = min_time_per_call_us(5, 1, || {
        sim_qs.previous_circulation_strength = vec![0.0; NR_WINGS * NR_STRIPS];

        run_time_steps(&mut sim_qs, 1, 1.0);
    });

    println!();
    println!("Induced velocity benchmark: {} wings x {} strips, {} wake panels",
        NR_WINGS, NR_STRIPS, wake.strengths.len()
    );
    println!("  Frozen wake, induced velocities (per solver iteration): {:>12.3} us", t_matvec);
    println!("  Frozen wake, fixed velocities (per time step):         {:>12.3} us", t_fixed);
    println!("  Frozen wake, variable factors (per time step):         {:>12.3} us", t_variable);
    println!("  Dynamic simulation, full time step:                    {:>12.3} us", t_dynamic_step);
    println!("  Quasi-steady iterative simulation, full time step:     {:>12.3} us", t_quasi_steady_step);
    println!();
}
