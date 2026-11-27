// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see seperate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)


use stormbird::actuator_line::ActuatorLine;
use stormbird::line_force_model::wing_internal_state::WingInternalState;

use stormath::spatial_vector::SpatialVector;

use std::path::Path;

#[cxx::bridge(namespace="stormbird_interface")]
mod ffi {
    struct DominatingLineWeight {
        line_index: usize,
        /// The projection weight of the dominating line element at the point
        weight: f64,
        /// The weight relative to the peak value of the line element, which is compared with the
        /// weight limits
        relative_weight: f64,
    }

    extern "Rust" {
        type CppActuatorLine;

        // ---- Constructors ----
        fn new_actuator_line_from_file(file_path: &str) -> *mut CppActuatorLine;

        // ---- Settings accessors ----
        fn use_point_sampling(&self) -> bool;
        fn sampling_weight_limit(&self) -> f64;
        fn projection_weight_limit(&self) -> f64;

        // ---- Setters and getters ----
        fn nr_span_lines(&self) -> usize;
        fn nr_wings(&self) -> usize;

        fn get_local_wing_angle(&self, index: usize) -> f64;
        fn set_local_wing_angle(&mut self, index: usize, angle: f64);

        fn get_wing_internal_state(&self, index: usize) -> f64;
        fn set_wing_internal_state(&mut self, index: usize, value: f64);

        fn get_ctrl_point_at_index(&self, index: usize) -> [f64; 3];

        fn get_weighted_velocity_sampling_integral_terms_for_cell(
            &self,
            line_index: usize,
            velocity: &[f64; 3],
            cell_center: &[f64; 3],
            cell_volume: f64,
        ) -> [f64; 4];

        fn set_velocity_at_index(&mut self, index: usize, velocity: [f64; 3]);

        // ---- Wake points of the dynamic lifting line correction ----
        fn nr_wake_points_to_sample(&self) -> usize;
        fn get_wake_point_at_index(&self, index: usize) -> [f64; 3];
        fn set_wake_point_velocity_at_index(&mut self, index: usize, velocity: [f64; 3]);

        fn dominating_line_element_and_weight_at_point(&self, point: &[f64; 3]) -> DominatingLineWeight;

        // ---- Force methods ----
        fn do_step(&mut self, time: f64, time_step: f64);
        fn update_controller(&mut self, time: f64, time_step: f64) -> bool;

        fn force_to_project(
            &self,
            line_index: usize,
            velocity: &[f64; 3]
        ) -> [f64; 3];

        // ---- Export data ----
        fn write_results(&self, folder_path: &str);
        fn write_correction_wake(&self, folder_path: &str, time_name: &str, time: f64);
    }
}

pub struct CppActuatorLine {
    model: ActuatorLine
}

/// Creates the actuator line model from a setup file.
///
/// # Panics
/// If the file can not be read, or if the settings are not valid. The panic message contains the
/// error from Stormbird. The panic can not unwind into the C++ code, so the program is stopped with
/// the message.
fn new_actuator_line_from_file(file_path: &str) -> *mut CppActuatorLine {
    let mut model = ActuatorLine::new_from_file(file_path).unwrap_or_else(|error| {
        panic!("Failed to create the actuator line model from the file {}: {}", file_path, error)
    });

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
    fn use_point_sampling(&self) -> bool {
        self.model.sampling_settings.use_point_sampling
    }

    fn sampling_weight_limit(&self) -> f64 {
        self.model.sampling_settings.weight_limit
    }

    fn projection_weight_limit(&self) -> f64 {
        self.model.projection_settings.weight_limit
    }

    fn nr_span_lines(&self) -> usize {
        self.model.line_force_model.nr_span_lines()
    }

    fn nr_wings(&self) -> usize {
        self.model.line_force_model.nr_wings()
    }

    fn get_ctrl_point_at_index(&self, index: usize) -> [f64; 3] {
        self.model.line_force_model.span_lines_global[index].ctrl_point().into()
    }

    fn get_local_wing_angle(&self, index: usize) -> f64 {
        self.model.line_force_model.local_wing_angles[index]
    }

    fn set_local_wing_angle(&mut self, index: usize, angle: f64) {
        self.model.line_force_model.local_wing_angles[index] = angle;

        self.model.line_force_model.update_global_data_representations();
    }

    /// The internal state of a wing, e.g., the rotational speed of a rotor sail. Only constant
    /// internal states are supported, which is what the controller sets.
    fn get_wing_internal_state(&self, index: usize) -> f64 {
        match &self.model.line_force_model.internal_state[index] {
            WingInternalState::Constant(value) => *value,
            WingInternalState::SpanVarying(_) => panic!(
                "Internal states that vary along the span are not supported in the CFD interface"
            ),
        }
    }

    fn set_wing_internal_state(&mut self, index: usize, value: f64) {
        self.model.line_force_model.set_wing_internal_state(index, WingInternalState::Constant(value));
    }

    fn get_weighted_velocity_sampling_integral_terms_for_cell(
        &self,
        line_index: usize,
        velocity: &[f64; 3],
        cell_center: &[f64; 3],
        cell_volume: f64,
    ) -> [f64; 4] {
        let (numerator, denominator) = self.model.get_weighted_velocity_sampling_integral_terms_for_cell(
            line_index, SpatialVector::from(*velocity), SpatialVector::from(*cell_center), cell_volume
        );

        [numerator[0], numerator[1], numerator[2], denominator]
    }

    fn set_velocity_at_index(&mut self, index: usize, velocity: [f64; 3]) {
        self.model.ctrl_points_velocity[index] = SpatialVector::from(velocity);
    }

    /// The number of points in the wake of the lifting line correction where the velocity must be
    /// sampled. Zero unless the correction uses the dynamic wake, and before the first time step.
    fn nr_wake_points_to_sample(&self) -> usize {
        self.model.nr_wake_points_to_sample()
    }

    fn get_wake_point_at_index(&self, index: usize) -> [f64; 3] {
        self.model.wake_points_to_sample()[index].into()
    }

    /// Sets the sampled velocity at a wake point. Values that are not finite are ignored, so that
    /// the previous value is kept.
    fn set_wake_point_velocity_at_index(&mut self, index: usize, velocity: [f64; 3]) {
        self.model.set_wake_point_velocity_at_index(index, SpatialVector::from(velocity));
    }

    fn dominating_line_element_and_weight_at_point(&self, point: &[f64; 3]) -> ffi::DominatingLineWeight {
        let (line_index, weight) = self.model.dominating_line_element_and_weight_at_point(
            SpatialVector::from(*point)
        );

        let relative_weight = self.model.relative_projection_weight(line_index, weight);

        ffi::DominatingLineWeight { line_index, weight, relative_weight }
    }

    pub fn do_step(&mut self, time: f64, time_step: f64) {
        self.model.do_step(time, time_step)
    }

    pub fn update_controller(&mut self, time: f64, time_step: f64) -> bool {
        self.model.update_controller(time, time_step)
    }

    pub fn force_to_project(
        &self,
        line_index: usize,
        velocity: &[f64; 3]
    ) -> [f64; 3] {
        let body_force = self.model.force_to_project_at_cell(
            line_index,
            SpatialVector::from(*velocity)
        );

        body_force.into()
    }

    /// Writes the results to the given folder. Errors when writing are printed as a warning, so
    /// that the simulation can continue.
    pub fn write_results(&self, folder_path: &str) {
        if let Err(error) = self.model.write_results(folder_path) {
            eprintln!(
                "Warning: failed to write the actuator line results to {}: {}",
                folder_path, error
            );
        }
    }

    /// Writes the wake of the lifting line correction as a legacy VTK file in the given folder,
    /// with the name `correction_wake_<time_name>.vtk`, and adds it to the ParaView series file
    /// `correction_wake.vtk.series` in the same folder, so that the files can be opened as one time
    /// series. Entries in the series file at or after the given time are replaced, e.g., after a
    /// restart. Does nothing if the correction has no panel wake. Errors are printed as a warning,
    /// so that the simulation can continue.
    pub fn write_correction_wake(&self, folder_path: &str, time_name: &str, time: f64) {
        if let Err(error) = self.try_write_correction_wake(folder_path, time_name, time) {
            eprintln!(
                "Warning: failed to write the lifting line correction wake to {}: {}",
                folder_path, error
            );
        }
    }

    fn try_write_correction_wake(
        &self,
        folder_path: &str,
        time_name: &str,
        time: f64
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(content) = self.model.correction_wake_as_vtk(true) else {
            return Ok(());
        };

        let folder = Path::new(folder_path);

        std::fs::create_dir_all(folder)?;

        let file_name = format!("correction_wake_{}.vtk", time_name);

        std::fs::write(folder.join(&file_name), content)?;

        let series_path = folder.join("correction_wake.vtk.series");

        let mut files: Vec<serde_json::Value> = match std::fs::read_to_string(&series_path) {
            Ok(text) => {
                let series: serde_json::Value = serde_json::from_str(&text)?;

                series["files"].as_array().cloned().unwrap_or_default()
            },
            Err(_) => Vec::new(),
        };

        files.retain(|entry| entry["time"].as_f64().is_some_and(|entry_time| entry_time < time));

        files.push(serde_json::json!({"name": file_name, "time": time}));

        let series = serde_json::json!({"file-series-version": "1.0", "files": files});

        std::fs::write(&series_path, serde_json::to_string_pretty(&series)?)?;

        Ok(())
    }
}
