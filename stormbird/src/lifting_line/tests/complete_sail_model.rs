// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Tests of the complete sail model, with and without a controller

use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

use crate::controller::builder::ControllerBuilder;
use crate::controller::measurements::SpanwiseMeasurementBuilder;
use crate::controller::set_points::ControllerSetPoints;
use crate::lifting_line::complete_sail_model::CompleteSailModel;
use crate::lifting_line::complete_sail_model::builder::CompleteSailModelBuilder;
use crate::lifting_line::simulation_builder::{SimulationBuilder, SimulationSettings};
use crate::line_force_model::builder::LineForceModelBuilder;
use crate::line_force_model::builder::single_wing::WingBuilder;
use crate::section_models::SectionModel;
use crate::section_models::foil::Foil;
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
