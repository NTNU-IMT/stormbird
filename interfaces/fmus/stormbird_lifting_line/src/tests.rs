// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Tests of the FMU logic, by calling the FMU functions directly on the struct

use super::*;

use std::path::Path;

use serde_json::{json, Value};

use stormbird::lifting_line::simulation_builder::{SimulationBuilder, SimulationSettings};
use stormbird::line_force_model::builder::LineForceModelBuilder;
use stormbird::line_force_model::builder::single_wing::WingBuilder;
use stormbird::section_models::SectionModel;
use stormbird::section_models::foil::Foil;

const TIME_STEP: f64 = 0.1;

/// Lifting line simulation with the given number of sails
fn simulation_builder(nr_sails: usize) -> SimulationBuilder {
    let mut line_force_model = LineForceModelBuilder::new(8);

    for i in 0..nr_sails {
        let x = 20.0 * i as f64;

        line_force_model.add_wing(WingBuilder {
            section_points: vec![
                SpatialVector::new(x, 0.0, 5.0),
                SpatialVector::new(x, 0.0, 35.0),
            ],
            chord_vectors: vec![SpatialVector::new(8.0, 0.0, 0.0); 2],
            section_model: SectionModel::SingleElementFoil(Foil::default()),
            ..Default::default()
        });
    }

    SimulationBuilder::new(line_force_model, SimulationSettings::default())
}

/// Parameters with a sail model without controller and wind environment, which are optional
fn parameters(nr_sails: usize) -> Value {
    json!({
        "sail_model": {
            "lifting_line_simulation": serde_json::to_value(simulation_builder(nr_sails)).unwrap()
        },
        "angles_in_degrees": true,
    })
}

/// Writes the content to a file in a temporary folder that is unique for each test
fn write_file(test_name: &str, content: &str) -> PathBuf {
    let folder = std::env::temp_dir().join("stormbird_lifting_line_fmu_tests").join(test_name);

    std::fs::create_dir_all(&folder).unwrap();

    let path = folder.join("parameters.json");

    std::fs::write(&path, content).unwrap();

    path
}

/// Creates the FMU with the parameters file at the given path, and exits the initialization mode
fn initialized_fmu(parameters_path: &Path) -> StormbirdLiftingLine {
    let mut fmu = StormbirdLiftingLine {
        parameters_path: parameters_path.to_string_lossy().to_string(),
        wind_velocity: 8.0,
        wind_direction_coming_from: 90.0,
        local_wing_angle_1: 10.0,
        local_wing_angle_2: 10.0,
        ..Default::default()
    };

    fmu.exit_initialization_mode();

    fmu
}

fn run_steps(fmu: &mut StormbirdLiftingLine, nr_steps: usize) {
    for i in 0..nr_steps {
        fmu.do_step(i as f64 * TIME_STEP, TIME_STEP);
    }
}

fn force_magnitude(fmu: &StormbirdLiftingLine) -> f64 {
    SpatialVector::new(fmu.force_x, fmu.force_y, fmu.force_z).length()
}

#[test]
fn valid_input_gives_forces_and_no_error() {
    let path = write_file("valid_input", &parameters(2).to_string());

    let mut fmu = initialized_fmu(&path);

    assert_eq!(fmu.error_code, error_codes::OK);

    run_steps(&mut fmu, 5);

    assert_eq!(fmu.error_code, error_codes::OK);
    assert!(force_magnitude(&fmu) > 0.0);
    assert!(fmu.force_sail_2_y.abs() > 0.0);
    assert_eq!(fmu.force_sail_3_y, 0.0);
    assert!(fmu.velocity_measurement_1 > 0.0);
    assert!(fmu.angle_of_attack_measurement_1.abs() > 1.0, "The angle should be in degrees");
}

#[test]
fn missing_parameters_file_gives_io_error() {
    let mut fmu = initialized_fmu(Path::new("this_file_does_not_exist.json"));

    assert_eq!(fmu.error_code, 1);

    run_steps(&mut fmu, 2);

    assert_eq!(fmu.error_code, 1);
    assert_eq!(force_magnitude(&fmu), 0.0);
}

#[test]
fn invalid_json_gives_parse_error() {
    let path = write_file("invalid_json", "{ this is not json");

    assert_eq!(initialized_fmu(&path).error_code, 2);

    let mut unknown_field = parameters(2);
    unknown_field["unknown_field"] = json!(1.0);

    let path = write_file("unknown_field", &unknown_field.to_string());

    assert_eq!(initialized_fmu(&path).error_code, 2);
}

#[test]
fn invalid_model_gives_invalid_input_error() {
    let mut invalid = parameters(2);

    invalid["sail_model"]["lifting_line_simulation"]["line_force_model"]["wing_builders"][0]
        ["section_points"] = json!([{"x": 0.0, "y": 0.0, "z": 0.0}]);

    let path = write_file("invalid_model", &invalid.to_string());

    assert_eq!(initialized_fmu(&path).error_code, 3);
}

#[test]
fn too_many_sails_gives_an_error() {
    let path = write_file("too_many_sails", &parameters(MAX_NR_SAILS + 1).to_string());

    assert_eq!(initialized_fmu(&path).error_code, error_codes::TOO_MANY_SAILS);
}

#[test]
fn step_without_initialization_gives_an_error() {
    let mut fmu = StormbirdLiftingLine::default();

    fmu.do_step(0.0, TIME_STEP);

    assert_eq!(fmu.error_code, error_codes::MODEL_NOT_INITIALIZED);
}

#[test]
fn errors_during_simulation_are_latched() {
    let path = write_file("latched_error", &parameters(2).to_string());

    let mut fmu = initialized_fmu(&path);

    run_steps(&mut fmu, 2);

    assert!(force_magnitude(&fmu) > 0.0);

    fmu.wind_velocity = f64::NAN;

    run_steps(&mut fmu, 1);

    assert_eq!(fmu.error_code, error_codes::NON_FINITE_INPUT);
    assert_eq!(force_magnitude(&fmu), 0.0);
    assert_eq!(fmu.velocity_measurement_1, 0.0);

    let error_code = fmu.error_code;

    // The model is not simulated again, even if the input is valid
    fmu.wind_velocity = 8.0;

    run_steps(&mut fmu, 2);

    assert_eq!(fmu.error_code, error_code);
    assert_eq!(force_magnitude(&fmu), 0.0);
}

#[test]
fn controller_sets_the_wing_angles() {
    let mut with_controller = parameters(1);

    with_controller["sail_model"]["controller"] = json!({
        "set_points": [{
            "apparent_wind_directions_data": [-3.2, 3.2],
            "angle_of_attack_data": [0.2, 0.2]
        }]
    });

    let path = write_file("controller", &with_controller.to_string());

    let mut fmu = initialized_fmu(&path);

    run_steps(&mut fmu, 3);

    assert_eq!(fmu.error_code, error_codes::OK);

    let local_wing_angle = fmu.sail_model.as_ref().unwrap()
        .lifting_line_simulation.line_force_model.local_wing_angles[0];

    assert!((local_wing_angle - 10_f64.to_radians()).abs() > 1e-3);
}

#[test]
fn panics_during_simulation_are_caught() {
    let path = write_file("panic", &parameters(2).to_string());

    let mut fmu = initialized_fmu(&path);

    // Corrupt the internal state of the model, so that the next step panics on an index out of
    // bounds
    fmu.sail_model.as_mut().unwrap()
        .lifting_line_simulation.line_force_model.local_wing_angles.clear();

    run_steps(&mut fmu, 1);

    assert_eq!(fmu.error_code, error_codes::PANIC_DURING_TIME_STEP);
    assert!(fmu.sail_model.is_none());
}
