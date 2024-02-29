// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Tests for the dynamic simulation capabilities of the lifting line module

use std::f64::consts::PI;

use crate::lifting_line::prelude::*;
use crate::lifting_line::simulation::{ 
    SimulationBuilder,
    SimulationMode,
    SteadySettings,
};

use super::test_setup::RectangularWing;

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

    let mut sim = SimulationBuilder::new (
        wing_builder,
        SimulationMode::QuasiSteady(SteadySettings::default()),
    ).build();

    let freestream_velocity = Vec3::new(1.2, 0.0, 0.0);

    let vel_magnitude = (freestream_velocity).length();
    
    let force_factor = 0.5 * aspect_ratio * vel_magnitude.powi(2) * sim.line_force_model.density;

    let time_step = 0.1;
    let period = 2.0;
    let frequency = 2.0 * PI / period;

    let rotation = Vec3::default();

    let amplitude = 0.23;

    for i in 1..20 {
        
        let time = (i as f64) * time_step;

        let translation_y = amplitude * (time * frequency).sin();
        let velocity_y = amplitude * frequency * (time * frequency).cos();

        let translation = Vec3::new(0.0, translation_y, 0.0);

        let input_state = InputState {
            freestream_velocity,
            translation,
            rotation,
        };

        let result = sim.do_step(time, time_step, input_state);
    
        let cl = result.integrated_forces_sum().y / force_factor;
    
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

    let mut sim = SimulationBuilder::new (
        wing_builder,
        SimulationMode::QuasiSteady(
            SteadySettings::default()
        ),
    ).build();

    let freestream_velocity = Vec3::new(10.2, 0.0, 0.0);

    let period = 2.0;
    let nr_time_steps_per_period = 20;
    let time_step = period / nr_time_steps_per_period as f64;

    let frequency = 2.0 * PI / period;

    let translation = Vec3::default();

    let amplitude = 5.0_f64.to_radians();

    for i in 1..nr_time_steps_per_period {
        let time = (i as f64) * time_step;

        let rotation = Vec3 {
            x: amplitude * (frequency * time).sin(),
            y: 0.0,
            z: 0.0,
        };

        let rotation_vel_x = frequency * amplitude * (frequency * time).cos();

        let input_state = InputState {
            freestream_velocity,
            translation,
            rotation,
        };

        let result = sim.do_step(time, time_step, input_state);
        
        
        let moment_in_x = result.integrated_moments_sum().x;
    
        assert!(
            moment_in_x * rotation_vel_x <= 0.0, 
            "Moment in x direction is not opposing the motion. Moment = {}, rotation velocity = {}", 
            moment_in_x,
            rotation_vel_x
        );
    }
}

#[test]
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

    let line_force_model = RectangularWing {
        aspect_ratio,
        cl_zero_angle,
        angle_of_attack,
        negative_span_orientation: true,
        ..Default::default()
    }.build().build();

    let freestream_velocity = Vec3::new(1.2, 0.0, 0.0);

    let mut previous_rotation = Vec3::default();
    let mut two_previous_rotation = Vec3::default();

    for i_t in 1..20 {
        let time = (i_t as f64) * time_step;

        let rotation_x     = amplitude * (frequency * time).sin();
        let rotation_vel_x = frequency * amplitude * (frequency * time).cos();

        let rotation = Vec3::new(rotation_x, 0.0, 0.0);
        let velocity = Vec3::new(rotation_vel_x, 0.0, 0.0);

        let rotation_velocity = (3.0 * rotation - 4.0 * previous_rotation + two_previous_rotation) / (2.0 * time_step);
        
        let velocity_input = VelocityInput {
            freestream: freestream_velocity,
            translation: Vec3::default(),
            rotation: rotation_velocity,
        };

        let ctrl_points = line_force_model.ctrl_points();
        
        let ctrl_point_velocity_est = velocity_input.felt_velocity_at_points(&ctrl_points);

        for i in 0..ctrl_points.len() {
            let velocity_local = velocity.cross(ctrl_points[i]);

            if velocity_local.y.abs() < 1e-6 {
                continue;
            }

            if i_t > 1 {
                let error = (velocity_local.y + ctrl_point_velocity_est[i].y).abs() / velocity_local.y.abs();

                assert!(error < 0.06, "Error in rotational velocity estimation at ctrl point {} = {}", i, error);
            }
        }

        two_previous_rotation = previous_rotation;
        previous_rotation = rotation;
    }
}