// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see seperate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)


use stormbird::actuator_line::ActuatorLine;

use math_utils::spatial_vector::SpatialVector;

#[cxx::bridge(namespace="stormbird_interface")]
mod ffi {
    extern "Rust" {
        type CppActuatorLine;

        // ---- Constructors ----
        fn new_actuator_line_from_file(file_path: &str) -> *mut CppActuatorLine;
        
        // ---- Setters and getters ----
        fn nr_span_lines(&self) -> usize;
        fn get_ctrl_point_at_index(&self, index: usize) -> [f64; 3];

        fn get_weighted_velocity_integral_terms_for_cell(
            &self,
            line_index: usize,
            velocity: &[f64; 3],
            cell_center: &[f64; 3],
            cell_volume: f64,
        ) -> [f64; 4];

        fn set_velocity_at_index(&mut self, index: usize, velocity: [f64; 3]);

        fn dominating_line_element_index_at_point(&self, point: &[f64; 3]) -> usize;

        // ---- Force methods ----        
        fn calculate_result(&mut self, time_step: f64);
        fn distributed_body_force_at_point(&self, point: &[f64; 3]) -> [f64; 3];
        fn summed_projection_weights_at_point(&self, point: &[f64; 3]) -> f64;
        
        // ---- Export data ----
        fn write_results(&self);
    }
}

pub struct CppActuatorLine {
    model: ActuatorLine
}

fn new_actuator_line_from_file(file_path: &str) -> *mut CppActuatorLine {
    let mut model = ActuatorLine::new_from_file(file_path);

    // TODO: this is currently a hack. The density needs to be set to one for incompressible, 
    // single-phase, flow, but should ideally take in the values from the CFD simulations in cases 
    // where the density might vary. This needs an update to handle such cases.
    model.line_force_model.density = 1.0;

    Box::into_raw(
        Box::new(
            CppActuatorLine{model}
        )
    )
}

impl CppActuatorLine {
    fn nr_span_lines(&self) -> usize {
        self.model.line_force_model.nr_span_lines()
    }

    fn get_ctrl_point_at_index(&self, index: usize) -> [f64; 3] {
        self.model.line_force_model.span_lines()[index].ctrl_point().into()
    }

    fn get_weighted_velocity_integral_terms_for_cell(
        &self,
        line_index: usize,
        velocity: &[f64; 3],
        cell_center: &[f64; 3],
        cell_volume: f64,
    ) -> [f64; 4] {
        let (numerator, denominator) = self.model.get_weighted_velocity_integral_terms_for_cell(
            line_index, SpatialVector::<3>::from(*velocity), SpatialVector::<3>::from(*cell_center), cell_volume
        );

        [numerator[0], numerator[1], numerator[2], denominator]
    }

    fn set_velocity_at_index(&mut self, index: usize, velocity: [f64; 3]) {
        self.model.ctrl_points_velocity[index] = SpatialVector::<3>::from(velocity);
    }

    fn dominating_line_element_index_at_point(&self, point: &[f64; 3]) -> usize {
        self.model.dominating_line_element_index_at_point(SpatialVector::<3>::from(*point))
    }

    pub fn calculate_result(&mut self, time_step: f64) {
        self.model.calculate_and_add_result(time_step)
    }

    pub fn distributed_body_force_at_point(&self, point: &[f64; 3]) -> [f64; 3] {
        let body_force = self.model.distributed_body_force_at_point(SpatialVector::<3>::from(*point));

        body_force.into()
    }

    pub fn summed_projection_weights_at_point(&self, point: &[f64; 3]) -> f64 {
        self.model.summed_projection_weights_at_point(SpatialVector::<3>::from(*point))
    }

    pub fn write_results(&self) {
        self.model.write_results();
    }
}
