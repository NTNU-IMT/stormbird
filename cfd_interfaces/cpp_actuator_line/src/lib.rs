// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see seperate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use stormbird::actuator_line::ActuatorLine;
use stormbird::vec3::Vec3 as Vec3Rust;

#[cxx::bridge(namespace="stormbird_interface")]
mod ffi {
    struct Vec3 {
        x: f64,
        y: f64,
        z: f64
    }

    extern "Rust" {
        type CppActuatorLine;

        fn new_actuator_line_from_file(file_path: &str) -> *mut CppActuatorLine;
        fn clear_cell_information(&mut self);
        fn add_cell_information(&mut self, center: &Vec3, velocity: Vec3, volume: f64);
        fn nr_sampling_span_lines(&self) -> usize;
        fn get_velocity_sampling_numerator(&self, index: usize) -> Vec3;
        fn get_velocity_sampling_denominator(&self, index: usize) -> f64;
        fn set_velocity_sampling_numerator(&mut self, index: usize, value: Vec3);
        fn set_velocity_sampling_denominator(&mut self, index: usize, value: f64);
        fn calculate_result(&mut self);
        fn distributed_body_force_at_point(&self, point: &Vec3) -> Vec3;
        fn distributed_body_force_weight_at_point(&self, point: &Vec3) -> f64;
        fn write_results(&self);
    }
}

use ffi::Vec3;

pub struct CppActuatorLine {
    model: ActuatorLine
}

impl Vec3 {
    fn to_rust(&self) -> Vec3Rust {
        Vec3Rust::new(self.x, self.y, self.z)
    }

    fn from_rust(vec: Vec3Rust) -> Self {
        Self {
            x: vec.x,
            y: vec.y,
            z: vec.z
        }
    }
}

fn new_actuator_line_from_file(file_path: &str) -> *mut CppActuatorLine {
    let mut model = ActuatorLine::new_from_file(file_path);

    // TODO: this is currently a hack. The density needs to be one for actuator line simualtions, but should be 
    // based on the values in the CFD simulations. This needs an update
    model.force_model_sampling.density = 1.0;
    model.force_model_projection.density = 1.0;

    Box::into_raw(
        Box::new(
            CppActuatorLine{model}
        )
    )
}

impl CppActuatorLine {
    fn clear_cell_information(&mut self) {
        self.model.clear_cell_information();
    }

    fn add_cell_information(&mut self, center: &Vec3, velocity: Vec3, volume: f64) {
        self.model.add_cell_information(center.to_rust(), velocity.to_rust(), volume);
    }

    fn nr_sampling_span_lines(&self) -> usize {
        self.model.force_model_sampling.nr_span_lines()
    }

    fn get_velocity_sampling_numerator(&self, index: usize) -> Vec3 {
        Vec3::from_rust(self.model.velocity_sampling.numerator_integral[index])
    }

    fn get_velocity_sampling_denominator(&self, index: usize) -> f64 {
        self.model.velocity_sampling.denominator_integral[index]
    }

    fn set_velocity_sampling_numerator(&mut self, index: usize, value: Vec3) {
        self.model.velocity_sampling.numerator_integral[index] = value.to_rust();
    }

    fn set_velocity_sampling_denominator(&mut self, index: usize, value: f64) {
        self.model.velocity_sampling.denominator_integral[index] = value;
    }

    pub fn calculate_result(&mut self) {
        self.model.calculate_and_add_result()
    }

    pub fn distributed_body_force_at_point(&self, point: &Vec3) -> Vec3 {
        let body_force = self.model.distributed_body_force_at_point(point.to_rust());

        Vec3::from_rust(body_force)
    }

    pub fn distributed_body_force_weight_at_point(&self, point: &Vec3) -> f64 {
        self.model.distributed_body_force_weight_at_point(point.to_rust())
    }

    pub fn write_results(&self) {
        self.model.write_results();
    }
}
