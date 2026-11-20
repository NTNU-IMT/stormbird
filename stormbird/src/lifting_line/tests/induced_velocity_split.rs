// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Tests of the split of the induced velocities into self-induced and other-induced velocities in
//! the frozen wake. The split is compared against direct sums over all panels or horseshoe vortices,
//! where the wing of each panel is found from the line force model.

use crate::lifting_line::prelude::*;

use crate::lifting_line::simulation::Simulation;
use crate::lifting_line::simulation_builder::{
    SimulationBuilder,
    SimulationSettings,
    QuasiSteadySettings,
    DynamicSettings,
};
use crate::lifting_line::wake::dynamic_wake::builder::DynamicWakeBuilder;
use crate::lifting_line::wake::settings::QuasiSteadyWakeSettings;
use crate::lifting_line::singularity_elements::symmetry_condition::SymmetryCondition;

use stormath::type_aliases::Float;

use crate::test_utils::tolerance;

/// Three wings at different positions and with different number of line elements, close enough
/// to each other to give significant wing-to-wing interactions
fn line_force_model_builder() -> LineForceModelBuilder {
    let positions = [[-4.0, 2.0], [0.0, -1.0], [5.0, 1.5]];
    let nr_sections = [6, 8, 5];

    let mut line_force_model_builder = LineForceModelBuilder::new(6);

    for (position, nr_sections) in positions.iter().zip(nr_sections) {
        let wing_builder = WingBuilder {
            section_points: vec![
                SpatialVector::from([position[0], position[1], 1.0]),
                SpatialVector::from([position[0], position[1], 6.0]),
            ],
            chord_vectors: vec![SpatialVector::from([1.0, 0.0, 0.0]); 2],
            section_model: SectionModel::SingleElementFoil(Foil {
                cl_zero_angle: 0.5,
                ..Default::default()
            }),
            non_zero_circulation_at_ends: [false, false],
            nr_sections: Some(nr_sections),
            ..Default::default()
        };

        line_force_model_builder.add_wing(wing_builder);
    }

    line_force_model_builder
}

fn run_simulation(simulation_settings: SimulationSettings, nr_time_steps: usize) -> Simulation {
    let mut sim = SimulationBuilder::new(
        line_force_model_builder(),
        simulation_settings
    ).build().unwrap();

    let time_step = 0.25;
    let freestream_velocity = SpatialVector::from([4.0, 1.0, 0.0]);

    for i in 0..nr_time_steps {
        let nr_points = sim.get_freestream_velocity_points().len();

        sim.do_step(
            i as Float * time_step,
            time_step,
            &vec![freestream_velocity; nr_points]
        );
    }

    sim
}

/// Checks the self-induced and the other-induced velocities against the reference values
fn assert_split_is_equal(
    induced_velocities: &InducedVelocities,
    reference_self_induced: &[SpatialVector],
    reference_other_induced: &[SpatialVector],
) {
    let mut max_value: Float = 0.0;

    for i in 0..induced_velocities.len() {
        max_value = max_value
            .max(reference_self_induced[i].length())
            .max(reference_other_induced[i].length());
    }

    assert!(max_value > 1e-3, "The test case should have non-zero induced velocities");

    let tolerance = tolerance(1e-10, 1e-4) * max_value;

    for i in 0..induced_velocities.len() {
        let self_error = (induced_velocities.self_induced[i] - reference_self_induced[i]).length();
        let other_error = (induced_velocities.other_induced[i] - reference_other_induced[i]).length();

        assert!(
            self_error < tolerance && other_error < tolerance,
            "Control point {}: self-induced {:?} vs {:?}, other-induced {:?} vs {:?}",
            i,
            induced_velocities.self_induced[i], reference_self_induced[i],
            induced_velocities.other_induced[i], reference_other_induced[i]
        );
    }
}

/// Compares the split in the frozen wake against direct sums over all panels in the dynamic wake
fn check_dynamic_wake(symmetry_condition: SymmetryCondition, steady_state_strength_update: bool) {
    let settings = DynamicSettings {
        wake: DynamicWakeBuilder {
            nr_panels_per_line_element: 20,
            symmetry_condition,
            steady_state_strength_update,
            ..Default::default()
        },
        ..Default::default()
    };

    let sim = run_simulation(SimulationSettings::Dynamic(settings), 30);

    let WakeData::Dynamic(wake) = &sim.wake_data else {
        panic!("Expected a dynamic wake");
    };

    let line_force_model = &sim.line_force_model;
    let ctrl_points = &line_force_model.ctrl_points_global;
    let nr_span_lines = line_force_model.nr_span_lines();

    // The strength of the first panels, which is the unknown strength in the frozen wake
    let strengths: Vec<Float> = wake.strengths[..nr_span_lines].to_vec();

    let mut frozen_wake = FrozenWake::initialize(line_force_model.wing_indices.clone());

    frozen_wake.update_from_full_wake(ctrl_points, wake);
    frozen_wake.update_induced_velocities_at_control_points(&strengths);

    let mut reference_self_induced = vec![SpatialVector::default(); nr_span_lines];
    let mut reference_other_induced = vec![SpatialVector::default(); nr_span_lines];

    for (point_index, &ctrl_point) in ctrl_points.iter().enumerate() {
        let point_wing = line_force_model.wing_index_from_global(point_index);

        for panel_index in 0..wake.strengths.len() {
            let span_index = panel_index % nr_span_lines;

            // With the steady state strength update, the whole wake has the strength of the first
            // panels
            let strength = if steady_state_strength_update {
                strengths[span_index]
            } else {
                wake.strengths[panel_index]
            };

            let u_i = strength * wake.unit_strength_induced_velocity_from_panel_flat_index(
                panel_index,
                ctrl_point
            );

            if line_force_model.wing_index_from_global(span_index) == point_wing {
                reference_self_induced[point_index] += u_i;
            } else {
                reference_other_induced[point_index] += u_i;
            }
        }
    }

    assert_split_is_equal(
        &frozen_wake.induced_velocities_at_control_points,
        &reference_self_induced,
        &reference_other_induced
    );

    // The total is also checked against the induced velocity function of the wake
    if !steady_state_strength_update {
        let total = frozen_wake.induced_velocities_at_control_points.total();
        let total_wake = wake.induced_velocities(ctrl_points);

        for (u_i, u_i_wake) in total.iter().zip(&total_wake) {
            assert!((*u_i - *u_i_wake).length() < tolerance(1e-10, 1e-4) * u_i_wake.length().max(1.0));
        }
    }
}

#[test]
fn dynamic_wake_split_is_exact() {
    check_dynamic_wake(SymmetryCondition::NoSymmetry, false);
}

#[test]
fn dynamic_wake_split_is_exact_with_symmetry() {
    check_dynamic_wake(SymmetryCondition::Z, false);
}

#[test]
fn dynamic_wake_split_is_exact_with_steady_state_strength_update() {
    check_dynamic_wake(SymmetryCondition::NoSymmetry, true);
}

/// Compares the split in the frozen wake against direct sums over the horseshoe vortices in the
/// quasi-steady wake
fn check_quasi_steady_wake(symmetry_condition: SymmetryCondition) {
    let settings = QuasiSteadySettings {
        wake: QuasiSteadyWakeSettings {
            symmetry_condition,
            ..Default::default()
        },
        ..Default::default()
    };

    let mut sim = run_simulation(SimulationSettings::QuasiSteady(settings), 1);

    let line_force_model = sim.line_force_model.clone();
    let ctrl_points = &line_force_model.ctrl_points_global;
    let nr_span_lines = line_force_model.nr_span_lines();

    let strengths = sim.previous_circulation_strength.clone();

    let WakeData::QuasiSteady(wake) = &sim.wake_data else {
        panic!("Expected a quasi-steady wake");
    };

    sim.frozen_wake.update_from_quasi_steady_wake(ctrl_points, wake);
    sim.frozen_wake.update_induced_velocities_at_control_points(&strengths);

    let mut reference_self_induced = vec![SpatialVector::default(); nr_span_lines];
    let mut reference_other_induced = vec![SpatialVector::default(); nr_span_lines];

    for (point_index, &ctrl_point) in ctrl_points.iter().enumerate() {
        let point_wing = line_force_model.wing_index_from_global(point_index);

        for (vortex_index, &strength) in strengths.iter().enumerate() {
            let u_i = strength * wake.unit_strength_induced_velocity(vortex_index, ctrl_point);

            if line_force_model.wing_index_from_global(vortex_index) == point_wing {
                reference_self_induced[point_index] += u_i;
            } else {
                reference_other_induced[point_index] += u_i;
            }
        }
    }

    assert_split_is_equal(
        &sim.frozen_wake.induced_velocities_at_control_points,
        &reference_self_induced,
        &reference_other_induced
    );
}

#[test]
fn quasi_steady_wake_split_is_exact() {
    check_quasi_steady_wake(SymmetryCondition::NoSymmetry);
}

#[test]
fn quasi_steady_wake_split_is_exact_with_symmetry() {
    check_quasi_steady_wake(SymmetryCondition::Z);
}

#[test]
/// With a single wing, all induced velocities are self-induced
fn single_wing_has_no_other_induced_velocities() {
    let mut builder = line_force_model_builder();
    builder.wing_builders.truncate(1);

    let mut sim = SimulationBuilder::new(
        builder,
        SimulationSettings::Dynamic(DynamicSettings::default())
    ).build().unwrap();

    let freestream_velocity = SpatialVector::from([4.0, 1.0, 0.0]);

    for i in 0..10 {
        let nr_points = sim.get_freestream_velocity_points().len();

        sim.do_step(i as Float * 0.25, 0.25, &vec![freestream_velocity; nr_points]);
    }

    let induced_velocities = &sim.frozen_wake.induced_velocities_at_control_points;

    for i in 0..induced_velocities.len() {
        assert!(induced_velocities.self_induced[i].length() > 1e-3);
        assert_eq!(induced_velocities.other_induced[i], SpatialVector::default());
    }
}
