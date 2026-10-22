// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Tests for the induced velocities and export of the quasi-steady wake

use crate::lifting_line::prelude::*;
use crate::lifting_line::simulation::Simulation;
use crate::lifting_line::simulation_builder::{
    SimulationBuilder,
    SimulationSettings,
    QuasiSteadySettings,
};
use crate::lifting_line::wake::settings::QuasiSteadyWakeSettings;

use stormath::type_aliases::Float;

use super::test_setup::RectangularWing;

fn quasi_steady_simulation(symmetry_condition: SymmetryCondition) -> Simulation {
    let line_force_model_builder = RectangularWing {
        angle_of_attack: Float::from(5.0).to_radians(),
        nr_strips: 8,
        ..Default::default()
    }.build();

    SimulationBuilder {
        line_force_model: line_force_model_builder,
        simulation_settings: SimulationSettings::QuasiSteady(QuasiSteadySettings {
            wake: QuasiSteadyWakeSettings {
                symmetry_condition,
                ..Default::default()
            },
            ..Default::default()
        })
    }.build()
}

fn do_step(sim: &mut Simulation) {
    let freestream_velocity = SpatialVector::from([1.2, 0.0, 0.0]);

    let nr_points = sim.get_freestream_velocity_points().len();

    sim.do_step(0.0, 1.0, &vec![freestream_velocity; nr_points]);
}

#[test]
/// The induced velocities from the quasi-steady wake at the control points must be the same as
/// the induced velocities used by the solver, i.e., the frozen wake with the final circulation
/// strength.
fn quasi_steady_induced_velocities_match_the_solver() {
    for symmetry_condition in [SymmetryCondition::NoSymmetry, SymmetryCondition::Z] {
        let mut sim = quasi_steady_simulation(symmetry_condition);

        // No induced velocities before the first time step
        let ctrl_points = sim.line_force_model.ctrl_points_global.clone();

        for velocity in sim.induced_velocities(&ctrl_points) {
            assert_eq!(velocity.length(), 0.0);
        }

        do_step(&mut sim);

        let circulation_strength = sim.previous_circulation_strength.clone();

        let induced_velocities = sim.induced_velocities(&ctrl_points);

        let nr_span_lines = ctrl_points.len();

        for row_index in 0..nr_span_lines {
            let mut expected = SpatialVector::default();

            for col_index in 0..nr_span_lines {
                expected += circulation_strength[col_index] *
                    sim.frozen_wake.variable_velocity_factors[[row_index, col_index]];
            }

            assert!(expected.length() > 0.0);
            assert!(
                (induced_velocities[row_index] - expected).length() < 1e-12 * expected.length(),
                "Induced velocity at control point {} differs from the solver", row_index
            );
        }
    }
}

#[test]
fn quasi_steady_wake_can_be_exported_as_vtk() {
    let mut sim = quasi_steady_simulation(SymmetryCondition::NoSymmetry);

    let nr_span_lines = sim.line_force_model.nr_span_lines();

    // The wake is empty before the first time step, but still valid VTK data
    let empty = String::from_utf8(sim.wake_as_vtk(false)).unwrap();

    assert!(empty.contains("POINTS 0 float"));
    assert!(empty.contains("POLYGONS 0 0"));

    do_step(&mut sim);

    let text = String::from_utf8(sim.wake_as_vtk(false)).unwrap();

    assert!(text.starts_with("# vtk DataFile Version 3.0"));
    assert!(text.contains(&format!("POINTS {} float", 6 * nr_span_lines)));
    assert!(text.contains(&format!("POLYGONS {} {}", nr_span_lines, 7 * nr_span_lines)));
    assert!(text.contains(&format!("CELL_DATA {}", nr_span_lines)));

    // The strength values in the export are the circulation strength from the solver
    let strength_values: Vec<Float> = text.lines()
        .skip_while(|line| !line.starts_with("SCALARS strength"))
        .skip(2)
        .take(nr_span_lines)
        .map(|line| line.parse().unwrap())
        .collect();

    for (exported, strength) in strength_values.iter().zip(&sim.previous_circulation_strength) {
        assert!((exported - strength).abs() < 1e-6 * strength.abs().max(1.0));
    }

    // The binary version has the same header and size information
    let binary = sim.wake_as_vtk(true);
    let header = String::from_utf8_lossy(&binary[0..80]);

    assert!(header.contains("BINARY"));
}
