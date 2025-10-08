// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use crate::lifting_line::prelude::*;
use crate::lifting_line::simulation_builder::{ 
    SimulationBuilder,
    SimulationSettings,
    UnsteadySettings,
};

use stormath::type_aliases::Float;

#[test]
/// This tests whether the lift and drag coefficients are correct for a steady simulation of two 
/// sails when the "neglect_self_induced_velocity" flag is set to true. The sails are placed far
/// apart so that the induced velocities should be very close to zero. 
/// 
/// The test was created as a result of a bug where the lift and drag coefficients were not correct
/// with multiple sails, but correct with only one sail.
fn no_self_induced_velocity() {
    let nr_strips = 10;
    let diameter  = 1.0;
    let span      = 5.0;
    let velocity_mag = 1.2;

    let revolutions_per_second = -RotatingCylinder::revolutions_per_second_from_spin_ratio(
        2.0, diameter, velocity_mag
    );

    let mut line_force_model_builder = LineForceModelBuilder::new(nr_strips);
    
    /*line_force_model_builder.prescribed_circulation = Some(
        PrescribedCirculation{
            shape: PrescribedCirculationShape{
                outer_power: 0.25,
                ..Default::default()
            },
            ..Default::default()
        }
    );*/

    let chord_vector = SpatialVector::from([diameter, 0.0, 0.0]);

    let wing_x_positions = vec![
        -1.0 * span,
        1.0 * span,
    ];

    let wing_y_positions = vec![
        -10.0 * span,
        10.0 * span,
    ];

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
            section_model: SectionModel::RotatingCylinder(
                RotatingCylinder {
                    revolutions_per_second,
                    spin_ratio_data: vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0],
                    cd_data: vec![0.554, 0.674, 1.497, 2.877, 3.556, 3.816],
                    cl_data: vec![0.0, 1.889, 4.638, 6.794, 7.680, 7.950],
                    wake_angle_data: Some(vec![
                        0.0, Float::from(20.0).to_radians(), Float::from(25.0).to_radians(), 
                        Float::from(35.0).to_radians(), Float::from(45.0).to_radians(), Float::from(60.0).to_radians()
                    ]),
                    ..Default::default()
                }
            ),
            non_zero_circulation_at_ends: [false, false],
            ..Default::default()
        };
    
        line_force_model_builder.add_wing(wing_builder);
    }

    let wake = WakeBuilder {
        neglect_self_induced_velocities: true,
        ..Default::default()
    };

    let settings = UnsteadySettings{
        wake,
        ..Default::default()
    };
    
    let nr_time_steps = 100;
    let time_step = 0.5;

    let velocity = SpatialVector::from([velocity_mag, 0.0, 0.0]);

    let mut sim = SimulationBuilder::new(
        line_force_model_builder,
        SimulationSettings::Dynamic(settings)
    ).build();

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

    let cd_data = 1.497;
    let cl_data = 4.638;

    let cd_error = cd - cd_data;
    let cl_error = cl - cl_data;

    dbg!(cd_error.abs() / cd_data, cl_error.abs() / cl_data);

    assert!(cd_error.abs() / cd_data < 0.0005);
    assert!(cl_error.abs() / cl_data < 0.0001);

}