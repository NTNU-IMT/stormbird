// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Compare the result from different solvers

use crate::lifting_line::prelude::*;
use crate::lifting_line::simulation_builder::{
    SimulationBuilder,
    SimulationMode,
    SteadySettings,
    UnsteadySettings,
};

use super::test_setup::RectangularWing;

#[test]
fn coordinate_systems() {
    let aspect_ratio = 5.0;
    let cl_zero_angle = 1.2;
    let angle_of_attack = 2.0_f64.to_radians();

    let line_force_model_builder = RectangularWing {
        aspect_ratio,
        cl_zero_angle,
        angle_of_attack,
        nr_strips: 32,
        ..Default::default()
    }.build();

    let steady_settings  = SteadySettings::default();
    let dynamic_settings = UnsteadySettings::default();

    let nr_time_steps = 200;

    let time_step = 0.25;

    let velocity = SpatialVector([1.2, 0.0, 0.0]);

    let mut steady_sim = SimulationBuilder::new(
        line_force_model_builder.clone(),
        SimulationMode::QuasiSteady(steady_settings.clone())
    ).build(time_step, velocity);

    
}