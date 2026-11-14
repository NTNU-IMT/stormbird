// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Tests of the complete sail model, with and without a controller

use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

use crate::controller::builder::ControllerBuilder;
use crate::controller::measurements::SpanwiseMeasurementBuilder;
use crate::controller::set_points::{ControllerSetPoints, InternalStateSetPointType};
use crate::lifting_line::complete_sail_model::CompleteSailModel;
use crate::lifting_line::complete_sail_model::builder::CompleteSailModelBuilder;
use crate::lifting_line::simulation_builder::{SimulationBuilder, SimulationSettings};
use crate::line_force_model::builder::LineForceModelBuilder;
use crate::line_force_model::builder::single_wing::WingBuilder;
use crate::section_models::SectionModel;
use crate::section_models::foil::Foil;
use crate::section_models::rotating_cylinder::RotatingCylinder;
use crate::wind::inflow_corrections::{InflowCorrections, InflowCorrectionsSingleDirection};
use crate::wind::wind_condition::discretized_spectrum::DiscretizedSpectrum;
use crate::wind::environment::WindEnvironment;
use crate::wind::wind_condition::WindCondition;

const SHIP_VELOCITY: Float = 6.0;

fn simulation_builder() -> SimulationBuilder {
    let mut line_force_model = LineForceModelBuilder::new(10);

    for x in [-20.0, 20.0] {
        line_force_model.add_wing(WingBuilder {
            section_points: vec![
                SpatialVector::from([x, 0.0, 5.0]),
                SpatialVector::from([x, 0.0, 35.0]),
            ],
            chord_vectors: vec![SpatialVector::from([8.0, 0.0, 0.0]); 2],
            section_model: SectionModel::SingleElementFoil(Foil::default()),
            ..Default::default()
        });
    }

    SimulationBuilder::new(line_force_model, SimulationSettings::default())
}

fn controller_builder() -> ControllerBuilder {
    let set_points = ControllerSetPoints {
        apparent_wind_directions_data: vec![-Float::to_radians(180.0), Float::to_radians(180.0)],
        angle_of_attack_data: Some(vec![Float::to_radians(10.0); 2]),
        ..Default::default()
    };

    ControllerBuilder {
        set_points: vec![set_points; 2],
        spanwise_measurement: SpanwiseMeasurementBuilder::default(),
        time_steps_between_updates: 1,
        start_time: 0.0,
        moving_average_window_size: None,
        use_input_velocity_for_apparent_wind_direction: false,
    }
}

fn model(controller: Option<ControllerBuilder>) -> CompleteSailModel {
    CompleteSailModelBuilder {
        lifting_line_simulation: simulation_builder(),
        wind_environment: WindEnvironment::default(),
        controller,
        settings: Default::default(),
    }.build().unwrap()
}

fn beam_wind() -> WindCondition {
    WindCondition::new_constant(Float::to_radians(90.0), 8.0)
}

#[test]
fn controller_and_wind_environment_are_optional_in_json() {
    let builder = CompleteSailModelBuilder {
        lifting_line_simulation: simulation_builder(),
        wind_environment: WindEnvironment::default(),
        controller: None,
        settings: Default::default(),
    };

    let mut json: serde_json::Value = serde_json::to_value(&builder).unwrap();

    let object = json.as_object_mut().unwrap();
    object.remove("controller");
    object.remove("wind_environment");
    object.remove("settings");

    let model = CompleteSailModel::new_from_string(&json.to_string()).unwrap();

    assert!(!model.has_controller());
}

#[test]
fn steady_state_without_controller_uses_the_given_wing_angles() {
    let mut model = model(None);

    let local_wing_angles = vec![Float::to_radians(5.0), Float::to_radians(-3.0)];

    model.set_local_wing_angles(&local_wing_angles);

    let result_low_loading = model.simulate_steady_state_condition(&beam_wind(), SHIP_VELOCITY, 0.2);
    let result_high_loading = model.simulate_steady_state_condition(&beam_wind(), SHIP_VELOCITY, 1.0);

    assert_eq!(model.lifting_line_simulation.line_force_model.local_wing_angles, local_wing_angles);

    // The loading has no effect without a controller
    let force_low = result_low_loading.integrated_forces_sum();
    let force_high = result_high_loading.integrated_forces_sum();

    assert!(force_low.length() > 0.0);
    assert!((force_low - force_high).length() < 1e-6 * force_low.length());

    let optimal = model.simulate_optimal_steady_state_condition(&beam_wind(), SHIP_VELOCITY, 1.0);

    assert!((optimal.integrated_forces_sum() - force_high).length() < 1e-6 * force_high.length());
}

#[test]
fn steady_state_with_controller_changes_the_wing_angles() {
    let mut model = model(Some(controller_builder()));

    model.simulate_steady_state_condition(&beam_wind(), SHIP_VELOCITY, 1.0);

    let local_wing_angles = &model.lifting_line_simulation.line_force_model.local_wing_angles;

    assert!(local_wing_angles.iter().all(|angle| angle.abs() > Float::to_radians(1.0)));
}

#[test]
fn flow_measurements_are_available_without_controller() {
    let mut model = model(None);

    let result = model.do_step(0.0, 1.0, &beam_wind(), SHIP_VELOCITY);

    let measurements = model.flow_measurements_from_simulation_result(1.0, &result);

    assert_eq!(measurements.len(), model.get_number_of_sails());
    assert!(measurements.iter().all(|measurement| measurement.velocity > 0.0));
}

#[test]
fn scalar_and_vector_ship_velocity_give_the_same_freestream() {
    let model = model(None);

    let wind_condition = beam_wind();

    let from_scalar = model.freestream_velocity(&wind_condition, SHIP_VELOCITY, 0.0);
    let from_vector = model.freestream_velocity_with_ship_velocity_vector(
        &wind_condition,
        model.ship_velocity_vector(SHIP_VELOCITY),
        0.0
    );

    assert_eq!(from_scalar, from_vector);

    // The ship moves in the opposite direction of the zero direction vector, so the apparent wind
    // from the ship velocity alone is along the zero direction vector
    let no_wind = WindCondition::new_constant(0.0, 0.0);

    let ship_only = model.freestream_velocity(&no_wind, SHIP_VELOCITY, 0.0);

    let expected = SHIP_VELOCITY * model.wind_environment.zero_direction_vector;

    for velocity in ship_only.iter().take(model.lifting_line_simulation.line_force_model.nr_span_lines()) {
        assert!((*velocity - expected).length() < 1e-9);
    }
}

fn relative_difference(a: SpatialVector, b: SpatialVector) -> Float {
    (a - b).length() / a.length().max(b.length())
}

#[test]
/// Settings that are only meaningful in the time domain should not affect the steady state, and
/// the result should not depend on earlier conditions
fn steady_state_is_independent_of_time_domain_settings_and_history() {
    let mut time_domain_controller = controller_builder();

    time_domain_controller.moving_average_window_size = Some(5);
    time_domain_controller.time_steps_between_updates = 3;
    time_domain_controller.start_time = 100.0;

    for set_points in time_domain_controller.set_points.iter_mut() {
        set_points.max_local_wing_angle_change_rate = Some(0.001);
    }

    let starboard_wind = WindCondition::new_constant(Float::to_radians(90.0), 8.0);
    let port_wind = WindCondition::new_constant(Float::to_radians(-90.0), 8.0);

    let reference = model(Some(controller_builder()))
        .simulate_steady_state_condition(&port_wind, SHIP_VELOCITY, 1.0)
        .integrated_forces_sum();

    let mut model_with_history = model(Some(time_domain_controller));

    model_with_history.simulate_steady_state_condition(&starboard_wind, SHIP_VELOCITY, 1.0);

    let with_history = model_with_history
        .simulate_steady_state_condition(&port_wind, SHIP_VELOCITY, 1.0)
        .integrated_forces_sum();

    assert!(
        relative_difference(reference, with_history) < 1e-6,
        "reference: {:?}, with history: {:?}", reference, with_history
    );

    // The controller state is not changed by the steady state simulations
    let controller = model_with_history.controller.as_ref().unwrap();

    assert_eq!(controller.time_step_index, 0);
}

#[test]
fn steady_state_neglects_gusts() {
    let mut gusty_wind = beam_wind();

    gusty_wind.parallel_gust = Some(DiscretizedSpectrum {
        frequencies: vec![0.3],
        amplitudes: vec![3.0],
        phases: vec![0.5],
    });

    let steady = model(Some(controller_builder()))
        .simulate_steady_state_condition(&beam_wind(), SHIP_VELOCITY, 1.0)
        .integrated_forces_sum();

    let with_gusts = model(Some(controller_builder()))
        .simulate_steady_state_condition(&gusty_wind, SHIP_VELOCITY, 1.0)
        .integrated_forces_sum();

    assert!(relative_difference(steady, with_gusts) < 1e-6);
}

#[test]
/// The returned steady state result should be consistent with the final settings of the sails,
/// also for rotor sails where the controller sets the internal state from the measured velocity
fn steady_state_result_matches_the_final_sail_settings() {
    let mut line_force_model = LineForceModelBuilder::new(10);

    for x in [-20.0, 20.0] {
        line_force_model.add_wing(WingBuilder {
            section_points: vec![
                SpatialVector::from([x, 0.0, 5.0]),
                SpatialVector::from([x, 0.0, 35.0]),
            ],
            chord_vectors: vec![SpatialVector::from([5.0, 0.0, 0.0]); 2],
            section_model: SectionModel::RotatingCylinder(RotatingCylinder::default()),
            ..Default::default()
        });
    }

    let set_points = ControllerSetPoints {
        apparent_wind_directions_data: vec![-Float::to_radians(180.0), Float::to_radians(180.0)],
        internal_state_data: Some(vec![3.0, 3.0]),
        internal_state_set_point_type: InternalStateSetPointType::SectionModel,
        ..Default::default()
    };

    let mut model = CompleteSailModelBuilder {
        lifting_line_simulation: SimulationBuilder::new(line_force_model, SimulationSettings::default()),
        wind_environment: WindEnvironment::default(),
        controller: Some(ControllerBuilder {
            set_points: vec![set_points; 2],
            ..controller_builder()
        }),
        settings: Default::default(),
    }.build().unwrap();

    let result = model.simulate_steady_state_condition(&beam_wind(), SHIP_VELOCITY, 1.0);

    let freestream_velocity = model.freestream_velocity(&beam_wind(), SHIP_VELOCITY, 0.0);

    let rerun = model.lifting_line_simulation.do_step(0.0, 1.0, &freestream_velocity);

    assert!(
        relative_difference(result.integrated_forces_sum(), rerun.integrated_forces_sum()) < 1e-3,
        "result: {:?}, rerun: {:?}", result.integrated_forces_sum(), rerun.integrated_forces_sum()
    );
}

#[test]
/// The inflow corrections should be looked up with the apparent wind direction that includes the
/// linear motion of the sails, so that giving the ship velocity as a ship velocity, or as motion of
/// the sails, gives the same corrections
fn inflow_corrections_include_the_motion_of_the_sails() {
    let mut model = model(None);

    let nr_ctrl_points = model.lifting_line_simulation.line_force_model.nr_span_lines();

    let wing_indices = model.lifting_line_simulation.line_force_model.wing_indices.clone();

    let heights: Vec<Float> = model.lifting_line_simulation.line_force_model.ctrl_points_global
        .iter()
        .map(|point| point[2])
        .collect();

    // Corrections that are different for head wind and beam wind
    let single_direction = |magnitude: Float| InflowCorrectionsSingleDirection {
        height_values: heights.clone(),
        magnitude_corrections: vec![magnitude; nr_ctrl_points],
        angle_corrections: vec![0.0; nr_ctrl_points],
        wing_indices: wing_indices.clone(),
    };

    model.wind_environment.inflow_corrections = Some(InflowCorrections {
        apparent_wind_directions: vec![-Float::to_radians(180.0), 0.0, Float::to_radians(180.0)],
        corrections: vec![single_direction(0.5), single_direction(1.5), single_direction(0.5)],
    });

    let wind_condition = beam_wind();
    let ship_velocity_vector = model.ship_velocity_vector(SHIP_VELOCITY);

    let with_ship_velocity = model.freestream_velocity_with_ship_velocity_vector(
        &wind_condition,
        ship_velocity_vector,
        0.0
    );

    model.lifting_line_simulation.line_force_model.rigid_body_motion.velocity_linear =
        ship_velocity_vector;

    let with_sail_motion = model.freestream_velocity_with_ship_velocity_vector(
        &wind_condition,
        SpatialVector::default(),
        0.0
    );

    // The freestream with the sail motion does not include the ship velocity, so the corrected
    // velocities differ, but the correction factor should be the same
    let true_wind = model.wind_environment.steady_true_wind_velocity_vector_at_location(
        &wind_condition,
        model.lifting_line_simulation.line_force_model.ctrl_points_global[0]
    );

    let factor_ship_velocity = with_ship_velocity[0].length() / (true_wind - ship_velocity_vector).length();
    let factor_sail_motion = with_sail_motion[0].length() / true_wind.length();

    assert!(
        (factor_ship_velocity - factor_sail_motion).abs() < 1e-9,
        "factor with ship velocity: {}, with sail motion: {}", factor_ship_velocity, factor_sail_motion
    );
}
