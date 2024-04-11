// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use serde::{Serialize, Deserialize};

pub use fmu_from_struct::prelude::*;

use stormbird::math_utils::interpolation;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Data {
    time: Vec<f64>,
    output: Vec<f64>,
}

impl Data {
    fn from_file(file_path: &str) -> Self {
        let file = std::fs::File::open(file_path).unwrap();
        let reader = std::io::BufReader::new(file);
        
        serde_json::from_reader(reader).unwrap()
    }

    fn interpolate(&self, time: f64) -> f64 {
        interpolation::linear_interpolation(time, &self.time, &self.output)
    }
}

#[derive(Debug, Default, Clone, Fmu)]
#[fmi_version = 2]
pub struct DataOutput {
    #[parameter]
    pub file_path: String,
    #[output]
    pub output: f64,

    data: Option<Data>,
}

impl FmuFunctions for DataOutput {
    fn exit_initialization_mode(&mut self) {
        self.data = Some(Data::from_file(&self.file_path));
    }

    fn do_step(&mut self, current_time: f64, _time_step: f64) {
        if let Some(data) = &self.data {
            self.output = data.interpolate(current_time);
        }
    }
}