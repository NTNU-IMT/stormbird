
use fmu_from_struct::prelude::*;
use serde::{Deserialize, Serialize};
use csv::Reader;
use stormath::interpolation::linear_interpolation;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct MotionRecord {
    time: f64,
    x_position: f64,
    y_position: f64,
    z_position: f64,
    x_rotation: f64,
    y_rotation: f64,
    z_rotation: f64,
}

#[derive(Fmu, Debug, Clone, Default)]
#[fmu_from_struct(fmi_version = 2)]
pub struct TabulatedMotion {
    #[fmu_from_struct(parameter)]
    pub motion_file_path: String,
    #[fmu_from_struct(output)]
    pub x_position: f64,
    pub y_position: f64,
    pub z_position: f64,
    pub x_rotation: f64,
    pub y_rotation: f64,
    pub z_rotation: f64,

    time_data: Vec<f64>,
    x_position_data: Vec<f64>,
    y_position_data: Vec<f64>,
    z_position_data: Vec<f64>,
    x_rotation_data: Vec<f64>,
    y_rotation_data: Vec<f64>,
    z_rotation_data: Vec<f64>,
}

impl FmuFunctions for TabulatedMotion {
    fn exit_initialization_mode(&mut self) {
        let file = std::fs::File::open(&self.motion_file_path).unwrap();
        let mut reader = Reader::from_reader(file);

        for result in reader.deserialize() {
            let record: MotionRecord = result.unwrap();
            self.time_data.push(record.time);
            self.x_position_data.push(record.x_position);
            self.y_position_data.push(record.y_position);
            self.z_position_data.push(record.z_position);
            self.x_rotation_data.push(record.x_rotation);
            self.y_rotation_data.push(record.y_rotation);
            self.z_rotation_data.push(record.z_rotation);
        }
    }

    fn do_step(&mut self, current_time: f64, _time_step: f64) {
        self.x_position = linear_interpolation(current_time, &self.time_data, &self.x_position_data);
        self.y_position = linear_interpolation(current_time, &self.time_data, &self.y_position_data);
        self.z_position = linear_interpolation(current_time, &self.time_data, &self.z_position_data);
        self.x_rotation = linear_interpolation(current_time, &self.time_data, &self.x_rotation_data);
        self.y_rotation = linear_interpolation(current_time, &self.time_data, &self.y_rotation_data);
        self.z_rotation = linear_interpolation(current_time, &self.time_data, &self.z_rotation_data);
    }
}
