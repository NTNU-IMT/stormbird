// Copyright (C) 2026, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Tests for the viscous correction in the linearized solver

use crate::lifting_line::prelude::*;
use crate::lifting_line::simulation_builder::{
    SimulationBuilder,
    SimulationSettings,
    QuasiSteadySettings,
};
use crate::lifting_line::solvers::QuasiSteadySolverBuilder;
use crate::lifting_line::solvers::linearized::{Linearized, InducedVelocityCorrectionMethod};

use stormath::type_aliases::Float;
use crate::test_utils::tolerance;

/// Lift coefficients for a rectangular wing with a sharp stall, for the given angles of attack,
/// simulated with the given linearized solver
fn lift_coefficients(angles_of_attack_deg: &[Float], solver: &Linearized) -> Vec<Float> {
    lift_coefficients_with_settings(angles_of_attack_deg, solver, false)
}

/// Same as [`lift_coefficients`], but with the option to include the viscous lift in the
/// circulation
fn lift_coefficients_with_settings(
    angles_of_attack_deg: &[Float],
    solver: &Linearized,
    include_viscous_lift_in_the_circulation: bool
) -> Vec<Float> {
    let aspect_ratio = 5.0;
    let freestream_velocity = SpatialVector::from([1.0, 0.0, 0.0]);

    angles_of_attack_deg.iter().map(|&angle_of_attack_deg| {
        let chord_vector = SpatialVector::from([1.0, 0.0, 0.0]).rotate_around_axis(
            -angle_of_attack_deg.to_radians(), SpatialVector::unit_z()
        );

        let mut line_force_model = LineForceModelBuilder::new(20);

        line_force_model.add_wing(WingBuilder {
            section_points: vec![
                SpatialVector::from([0.0, 0.0, 0.0]),
                SpatialVector::from([0.0, 0.0, aspect_ratio]),
            ],
            chord_vectors: vec![chord_vector, chord_vector],
            section_model: SectionModel::SingleElementFoil(Foil {
                mean_positive_stall_angle: Float::to_radians(15.0),
                mean_negative_stall_angle: Float::to_radians(15.0),
                stall_range: Float::to_radians(1.0),
                ..Default::default()
            }),
            ..Default::default()
        });

        line_force_model.force_calculation_settings.include_viscous_lift_in_the_circulation =
            include_viscous_lift_in_the_circulation;

        let mut simulation = SimulationBuilder {
            line_force_model,
            simulation_settings: SimulationSettings::QuasiSteady(QuasiSteadySettings {
                solver: QuasiSteadySolverBuilder::Linearized(solver.clone()),
                ..Default::default()
            })
        }.build().unwrap();

        let nr_points = simulation.get_freestream_velocity_points().len();

        let result = simulation.do_step(0.0, 1.0, &vec![freestream_velocity; nr_points]);

        result.integrated_forces_sum()[1].abs() /
            simulation.line_force_model.total_force_factor(freestream_velocity.length())
    }).collect()
}

/// The largest amount that the lift coefficients are above the lift coefficients without viscous
/// corrections, relative to the lift coefficients without viscous corrections
fn max_relative_lift_above_linear(lift_coefficients: &[Float], linear_lift_coefficients: &[Float]) -> Float {
    lift_coefficients.iter()
        .zip(linear_lift_coefficients.iter())
        .map(|(cl, cl_linear)| (cl - cl_linear) / cl_linear)
        .fold(Float::NEG_INFINITY, Float::max)
}

#[test]
/// With the full correction of the induced velocities, the circulation is corrected with the angles
/// of attack from the linear solution, while the forces are computed with the angles of attack
/// after the correction. Right before stall, this gives both unstalled circulation and post-stall
/// lift, which shows up as a jump in the lift, above the lift without viscous corrections. The
/// default method, without the correction of the induced velocities, uses the same angles of attack
/// for both, and the viscous correction does not increase the lift.
fn viscous_correction_without_lift_jump_before_stall() {
    let angles_of_attack_deg: Vec<Float> = (0..=60).map(|i| 8.0 + 0.25 * i as Float).collect();

    let cl_linear = lift_coefficients(
        &angles_of_attack_deg,
        &Linearized { disable_viscous_corrections: true, ..Default::default() }
    );

    let cl_full_correction = lift_coefficients(
        &angles_of_attack_deg,
        &Linearized {
            induced_velocity_correction_method: InducedVelocityCorrectionMethod::FullCorrection,
            ..Default::default()
        }
    );

    let cl_default = lift_coefficients(&angles_of_attack_deg, &Linearized::default());

    let excess_full_correction = max_relative_lift_above_linear(&cl_full_correction, &cl_linear);
    let excess_default = max_relative_lift_above_linear(&cl_default, &cl_linear);

    dbg!(excess_full_correction, excess_default);

    assert!(
        excess_full_correction > 0.02,
        "Expected a jump in the lift with the full correction. Max relative excess: {}",
        excess_full_correction
    );

    assert!(
        excess_default < 0.01,
        "The viscous correction should not increase the lift. Max relative excess: {}",
        excess_default
    );
}

#[test]
/// When the viscous lift is included in the circulation, there is no separate viscous lift force,
/// so the viscous correction of the circulation must include the post-stall lift. Otherwise, the
/// post-stall lift is lost. Well above the stall angle, where most of the lift is post-stall lift,
/// the lift should therefore be similar with and without the viscous lift in the circulation.
fn viscous_lift_in_the_circulation_keeps_the_post_stall_lift() {
    let angles_of_attack_deg = [30.0, 45.0];
    let solver = Linearized::default();

    let cl_separate = lift_coefficients_with_settings(&angles_of_attack_deg, &solver, false);
    let cl_in_circulation = lift_coefficients_with_settings(&angles_of_attack_deg, &solver, true);

    for (cl_separate, cl_in_circulation) in cl_separate.iter().zip(cl_in_circulation.iter()) {
        assert!(*cl_separate > 0.1);

        assert!(
            (cl_in_circulation - cl_separate).abs() < 0.2 * cl_separate,
            "The post-stall lift is not kept when it is included in the circulation. \
            Separate viscous lift = {}, viscous lift in the circulation = {}",
            cl_separate, cl_in_circulation
        );
    }
}

#[test]
/// For a pitched wing, the freestream has a component along the span. The velocity in the result
/// of the linearized solver should not include this component, so that the angles of
/// attack and the forces are the same as for the iterative solver, which always removes it.
fn a_pitched_wing_gives_the_same_angles_of_attack_with_both_solvers() {
    use crate::lifting_line::solvers::iterative::QuasiSteadyIterativeBuilder;
    use super::test_setup::RectangularWing;

    let pitch = Float::to_radians(30.0);

    let results: Vec<SimulationResult> = [false, true].iter().map(|&iterative| {
        let mut builder = RectangularWing {
            aspect_ratio: 5.0,
            cl_zero_angle: 0.0,
            angle_of_attack: Float::to_radians(5.0),
            nr_strips: 32,
            ..Default::default()
        }.build();

        builder.rotation = SpatialVector::from([0.0, pitch, 0.0]);

        let mut settings = QuasiSteadySettings::default();

        if iterative {
            settings.solver = QuasiSteadySolverBuilder::Iterative(
                QuasiSteadyIterativeBuilder::default()
            );
        }

        let mut simulation = SimulationBuilder::new(
            builder,
            SimulationSettings::QuasiSteady(settings)
        ).build().unwrap();

        let freestream = vec![
            SpatialVector::from([10.0, 0.0, 0.0]);
            simulation.get_freestream_velocity_points().len()
        ];

        let mut result = simulation.do_step(0.0, 0.1, &freestream);

        for _ in 0..10 {
            result = simulation.do_step(0.0, 0.1, &freestream);
        }

        // The velocity in the result has no component along the span
        for (i, velocity) in result.force_input.velocity.iter().enumerate() {
            let span_direction = simulation.line_force_model.span_lines_global[i].direction();

            assert!(
                velocity.dot(span_direction).abs() < tolerance(1e-9, 1e-6) * velocity.length(),
                "iterative: {}, span velocity at {}: {}", iterative, i, velocity.dot(span_direction)
            );
        }

        result
    }).collect();

    let (linearized, iterative) = (&results[0], &results[1]);

    for i in 0..linearized.force_input.angles_of_attack.len() {
        let difference = (
            linearized.force_input.angles_of_attack[i] - iterative.force_input.angles_of_attack[i]
        ).to_degrees();

        assert!(
            difference.abs() < 0.05,
            "angle of attack at {}: linearized {}, iterative {}",
            i,
            linearized.force_input.angles_of_attack[i].to_degrees(),
            iterative.force_input.angles_of_attack[i].to_degrees()
        );
    }

    let force_linearized = linearized.integrated_forces_sum();
    let force_iterative = iterative.integrated_forces_sum();

    assert!(
        (force_linearized - force_iterative).length() < 0.01 * force_iterative.length(),
        "force: linearized {:?}, iterative {:?}", force_linearized, force_iterative
    );
}

#[test]
/// Control points with no velocity get zero circulation in the linearized solver. Before, a single
/// control point without velocity made the equation system singular, and gave NaN for all the
/// circulation and the forces.
fn zero_velocity_gives_zero_circulation() {
    use super::test_setup::RectangularWing;

    let simulation = || SimulationBuilder::new(
        RectangularWing { angle_of_attack: Float::to_radians(5.0), ..Default::default() }.build(),
        SimulationSettings::QuasiSteady(QuasiSteadySettings::default())
    ).build().unwrap();

    // No velocity anywhere
    let mut sim = simulation();
    let nr_points = sim.get_freestream_velocity_points().len();

    let result = sim.do_step(0.0, 0.1, &vec![SpatialVector::default(); nr_points]);

    assert!(result.force_input.circulation_strength.iter().all(|gamma| *gamma == 0.0));
    assert!(result.residual.is_finite());
    assert_eq!(result.integrated_forces_sum().length(), 0.0);

    // No velocity at the first control point only
    let mut sim = simulation();

    let mut freestream = vec![SpatialVector::from([10.0, 0.0, 0.0]); nr_points];
    freestream[0] = SpatialVector::default();

    let result = sim.do_step(0.0, 0.1, &freestream);

    let circulation = &result.force_input.circulation_strength;

    assert_eq!(circulation[0], 0.0);
    assert!(circulation.iter().all(|gamma| gamma.is_finite()), "{:?}", circulation);
    assert!(circulation[1..].iter().all(|gamma| *gamma != 0.0), "{:?}", circulation);

    let force = result.integrated_forces_sum();

    assert!(force[0].is_finite() && force[1].is_finite() && force[2].is_finite(), "{:?}", force);
    assert!(force.length() > 0.0);
}
