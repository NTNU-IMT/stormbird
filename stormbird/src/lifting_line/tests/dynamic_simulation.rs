// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Tests for the dynamic simulation capabilities of the lifting line module

use std::f64::consts::PI;

use crate::lifting_line::prelude::*;
use crate::lifting_line::simulation_builder::{ 
    SimulationBuilder,
    SimulationSettings,
    SteadySettings,
    UnsteadySettings,
};

use super::test_setup::RectangularWing;
use super::elliptic_wing_theory::EllipticWingTheory;

#[test]
/// tests that a dynamic forward motion gives the same result as with steady inflow.
fn right_force_magnitude_with_steady_forward_motion() {
    let aspect_ratio = 5.0;
    let cl_zero_angle = 0.5;
    let angle_of_attack = 0.0;
    let cl_2d = cl_zero_angle + 2.0 * PI * angle_of_attack;

    let theory = EllipticWingTheory {
        cl_2d,
        aspect_ratio
    };

    dbg!(theory.cl(), theory.cd());

    let wing_builder = RectangularWing {
        aspect_ratio,
        cl_zero_angle,
        angle_of_attack,
        negative_span_orientation: true,
        ..Default::default()
    }.build();

    let vel_magnitude = 5.2;

    let freestream_velocity = SpatialVector([vel_magnitude, 0.0, 0.0]);

    let steady_settings = SteadySettings::default();
    let dynamic_settings = UnsteadySettings::default();

    let _sim_settings_steady = SimulationSettings::QuasiSteady(steady_settings.clone());
    let sim_settings_dynamic = SimulationSettings::Dynamic(dynamic_settings.clone());

    let mut sim_translation = SimulationBuilder::new (
        wing_builder.clone(),
        sim_settings_dynamic.clone(),
    ).build();

    let mut sim_inflow = SimulationBuilder::new (
        wing_builder,
        sim_settings_dynamic.clone(),
    ).build();

    let freestream_velocity_points = sim_inflow.get_freestream_velocity_points();
    let input_freestream_velocity = vec![freestream_velocity; freestream_velocity_points.len()];
    let input_freestream_velocity_translation = vec![SpatialVector::<3>::default(); freestream_velocity_points.len()];

    let force_factor = 0.5 * aspect_ratio * vel_magnitude.powi(2) * sim_translation.line_force_model.density;

    let time_step = 0.1;
    for i in 1..20 {
        
        let time = (i as f64) * time_step;
        
        let translation = SpatialVector([
            -vel_magnitude * time, 
            0.0, 
            0.0
        ]);

        sim_translation
            .line_force_model
            .rigid_body_motion
            .update_translation_with_velocity_using_finite_difference(
                translation,
                time_step,
            );

        //sim_translation.line_force_model.rigid_body_motion.velocity_linear = -freestream_velocity;

        let result_translation = sim_translation.do_step(
            time, 
            time_step, 
            &input_freestream_velocity_translation
        );

        let result_inflow = sim_inflow.do_step(
            time, 
            time_step, 
            &input_freestream_velocity
        );
    
        let cl_translation = result_translation.integrated_forces_sum()[1] / force_factor;
        let cl_inflow = result_inflow.integrated_forces_sum()[1] / force_factor;

        let cl_difference = (cl_translation - cl_inflow).abs() / cl_inflow.abs();

        assert!(
            cl_difference < 0.00001,
            "The difference between the two simulations is too large. Difference = {}",
            cl_difference
        );

        dbg!(cl_translation, cl_inflow, cl_difference);
    }

}

#[test]
/// Tests whether a wing moving dynamically will create forces in the right direction. The forces
/// and moments from a motion should always oppose the motion. In other words, a symmetric wing that
/// move upwards should experience a downwards force and vice versa.
fn right_sign_of_the_force_when_translating() {
    let aspect_ratio = 5.0;
    let cl_zero_angle = 0.0;
    let angle_of_attack = 0.0;

    let wing_builder = RectangularWing {
        aspect_ratio,
        cl_zero_angle,
        angle_of_attack,
        negative_span_orientation: true,
        ..Default::default()
    }.build();

    let freestream_velocity = SpatialVector([1.2, 0.0, 0.0]);

    let vel_magnitude = (freestream_velocity).length();

    let time_step = 0.1;
    let period = 2.0;
    let frequency = 2.0 * PI / period;

    let amplitude = 0.23;

    let mut sim = SimulationBuilder::new (
        wing_builder,
        SimulationSettings::QuasiSteady(SteadySettings::default()),
    ).build();

    let freestream_velocity_points = sim.get_freestream_velocity_points();
    let input_freestream_velocity = vec![freestream_velocity; freestream_velocity_points.len()];

    let force_factor = 0.5 * aspect_ratio * vel_magnitude.powi(2) * sim.line_force_model.density;

    for i in 1..20 {
        
        let time = (i as f64) * time_step;

        let translation_y = amplitude * (time * frequency).sin();
        let velocity_y = amplitude * frequency * (time * frequency).cos();

        let translation = SpatialVector([0.0, translation_y, 0.0]);

        sim.line_force_model.rigid_body_motion.translation = translation;

        let result = sim.do_step(time, time_step, &input_freestream_velocity);
    
        let cl = result.integrated_forces_sum()[1] / force_factor;
    
        //dbg!(cl * velocity_y);
    
        assert!(
            cl * velocity_y <= 0.0,
            "Force in y direction is not opposing the motion. Cl = {}, velocity = {}",
            cl,
            velocity_y
        ); 
    }
}

#[test]
/// Tests whether a wing moving dynamically will create forces in the right direction. The forces
/// and moments from a motion should always oppose the motion. In other words, a symmetric wing that
/// move upwards should experience a downwards force and vice versa.
fn right_sign_of_the_moment_when_rotating() {
    let aspect_ratio = 5.0;
    let cl_zero_angle = 0.0;
    let angle_of_attack = 0.0;

    let wing_builder = RectangularWing {
        aspect_ratio,
        cl_zero_angle,
        angle_of_attack,
        negative_span_orientation: true,
        ..Default::default()
    }.build();

    
    let freestream_velocity = SpatialVector([10.2, 0.0, 0.0]);

    let period = 2.0;
    let nr_time_steps_per_period = 20;
    let time_step = period / nr_time_steps_per_period as f64;

    let mut sim = SimulationBuilder::new (
        wing_builder,
        SimulationSettings::QuasiSteady(
            SteadySettings::default()
        ),
    ).build();

    let frequency = 2.0 * PI / period;

    let amplitude = 5.0_f64.to_radians();

    let freestream_velocity_points = sim.get_freestream_velocity_points();
    let input_freestream_velocity = vec![freestream_velocity; freestream_velocity_points.len()];

    for i in 1..nr_time_steps_per_period {
        let time = (i as f64) * time_step;

        let rotation = SpatialVector([
            amplitude * (frequency * time).sin(),
            0.0,
            0.0,
        ]);

        let rotation_vel_x = frequency * amplitude * (frequency * time).cos();

        sim.line_force_model.rigid_body_motion.rotation = rotation;

        let result = sim.do_step(time, time_step, &input_freestream_velocity);
        
        
        let moment_in_x = result.integrated_moments_sum()[0];
    
        assert!(
            moment_in_x * rotation_vel_x <= 0.0, 
            "Moment in x direction is not opposing the motion. Moment = {}, rotation velocity = {}", 
            moment_in_x,
            rotation_vel_x
        );
    }
}

/*#[test]
/// tests the estimated ctrl point velocity when applying a rotation
fn rotational_velocity() {
    // wing settings
    let aspect_ratio = 5.0;
    let cl_zero_angle = 0.0;
    let angle_of_attack = 0.0;

    // motion settings
    let amplitude = 1.0_f64.to_radians();
    let time_step = 0.1;
    let period = 2.0;
    let frequency = 2.0 * PI / period;

    let mut line_force_model = RectangularWing {
        aspect_ratio,
        cl_zero_angle,
        angle_of_attack,
        negative_span_orientation: true,
        ..Default::default()
    }.build().build();

    let freestream_velocity = SpatialVector([1.2, 0.0, 0.0]);

    let mut motion_calculator = MotionDerivatives::new(&line_force_model);

    for i_t in 1..20 {
        let time = (i_t as f64) * time_step;

        let rotation_x     = amplitude * (frequency * time).sin();
        let rotation_vel_x = frequency * amplitude * (frequency * time).cos();

        let rotation = SpatialVector([rotation_x, 0.0, 0.0]);
        let velocity = SpatialVector([rotation_vel_x, 0.0, 0.0]);

        line_force_model.rigid_body_motion.rotation = rotation;

        let motion_velocity = motion_calculator.ctrl_point_velocity(&line_force_model, time_step);
        
        let ctrl_points = line_force_model.ctrl_points();

        let mut ctrl_point_velocity_est = vec![freestream_velocity; ctrl_points.len()];

        for i in 0..line_force_model.nr_span_lines() {
            ctrl_point_velocity_est[i] -= motion_velocity[i];
        }

        for i in 0..ctrl_points.len() {
            let velocity_local = velocity.cross(ctrl_points[i]);

            if velocity_local[1].abs() < 1e-6 {
                continue;
            }

            if i_t > 3 {
                let error = (velocity_local[1] + ctrl_point_velocity_est[i][1]).abs() / velocity_local[1].abs();

                assert!(error < 0.06, "Error in rotational velocity estimation at ctrl point {} = {}", i, error);
            }
        }
    
        motion_calculator.update(&ctrl_points, line_force_model.rigid_body_motion.rotation);
    }
}*/