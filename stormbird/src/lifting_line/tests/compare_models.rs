// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Compare the result from different solvers

use std::f64::consts::PI;

use crate::lifting_line::prelude::*;
use crate::lifting_line::simulation::{ 
    SimulationBuilder,
    SimulationMode,
    SteadySettings,
    UnsteadySettings,
}; 

use super::test_setup::RectangularWing;
use super::elliptic_wing_theory::EllipticWingTheory;

#[test]
fn steady_lift() {
    let aspect_ratio = 5.0;
    let cl_zero_angle = 0.4;
    let angle_of_attack = 2.0_f64.to_radians();

    let theory = EllipticWingTheory {
        cl_2d: cl_zero_angle + 2.0 * PI * angle_of_attack,
        aspect_ratio
    };

    let cl_theory = theory.cl();
    let cd_theory = theory.cd();

    let wing_builder = RectangularWing {
        aspect_ratio,
        cl_zero_angle,
        angle_of_attack,
        ..Default::default()
    }.build();

    let steady_settings  = SteadySettings::default();
    let dynamic_settings = UnsteadySettings::default();

    

    let nr_time_steps = 200;

    let time_step = 0.25;

    let velocity = Vec3::new(1.2, 0.0, 0.0);

    let mut steady_sim = SimulationBuilder::new(
        wing_builder.clone(),
        SimulationMode::QuasiSteady(steady_settings)
    ).build(time_step, velocity);

    let mut dynamic_sim = SimulationBuilder::new(
        wing_builder.clone(),
        SimulationMode::Dynamic(dynamic_settings)
    ).build(time_step, velocity);

    let force_factor = steady_sim.line_force_model.total_force_factor(velocity.length());

    let dynamic_velocity_points = dynamic_sim.get_freestream_velocity_points();
    let static_velocity_points = steady_sim.get_freestream_velocity_points();

    let dynamic_velocity_freestream: Vec<Vec3> = vec![velocity; dynamic_velocity_points.len()];
    let static_velocity_freestream: Vec<Vec3> = vec![velocity; static_velocity_points.len()];

    let result_steady  = steady_sim.do_step(0.0, time_step, &static_velocity_freestream);

    let cd_steady = result_steady.integrated_forces_sum().x / force_factor;
    let cl_steady = result_steady.integrated_forces_sum().y / force_factor;

    let mut cd_dynamic = 0.0;
    let mut cl_dynamic = 0.0;

    for i in 0..nr_time_steps {
        let time = (i as f64) * time_step;
        
        let result_dynamic = dynamic_sim.do_step(time, time_step, &dynamic_velocity_freestream);

        cd_dynamic = result_dynamic.integrated_forces_sum().x / force_factor;
        cl_dynamic = result_dynamic.integrated_forces_sum().y / force_factor;   
    }

    println!("Theory");
    dbg!(cd_theory, cl_theory);
    println!("Steady");
    dbg!(cd_steady, cl_steady);
    println!("Dynamic");
    dbg!(cd_dynamic, cl_dynamic);

    let steady_cl_error = (cl_theory - cl_steady).abs() / cl_theory.abs();
    let steady_cd_error = (cd_theory - cd_steady).abs() / cd_theory.abs();

    let dynamic_cl_error = (cl_theory - cl_dynamic).abs() / cl_theory.abs();
    let dynamic_cd_error = (cd_theory - cd_dynamic).abs() / cd_theory.abs();

    let allowable_cd_error = 0.05;
    let allowable_cl_error = 0.07;

    assert!(steady_cl_error < allowable_cl_error, "Steady cl error: {}", steady_cl_error);
    assert!(steady_cd_error < allowable_cd_error, "Steady cd error: {}", steady_cd_error);
    assert!(dynamic_cl_error < allowable_cl_error, "Dynamic cl error: {}", dynamic_cl_error);
    assert!(dynamic_cd_error < allowable_cd_error, "Dynamic cd error: {}", dynamic_cd_error);   
}