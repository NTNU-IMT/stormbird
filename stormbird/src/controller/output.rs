// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)


use serde::{Deserialize, Serialize};

use crate::io_utils::csv_data;
use stormath::type_aliases::Float;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ControllerOutput {
    pub local_wing_angles: Vec<Float>,
    pub internal_states: Vec<Float>,
}

impl ControllerOutput {
    pub fn as_csv_string(output_to_write: &Self) -> (String, String) {
        let mut header = String::new();
        let mut data = String::new();
        
        let nr_wings = output_to_write.local_wing_angles.len();
        
        for i in 0..nr_wings {
            if i > 0 {
                header.push(',');
                data.push(',');
            }
            
            header.push_str(&format!("local_wing_angle_{}", i));
            data.push_str(&format!("{:.6}", output_to_write.local_wing_angles[i]));
        }
        
        for i in 0..nr_wings {
            header.push(',');
            data.push(',');
            
            header.push_str(&format!("section_model_internal_state_{}", i));
            data.push_str(&format!("{:.6}", output_to_write.internal_states[i]));
        }
        
        (header, data)
    }

    pub fn write_to_csv_file(output_to_write: &Self, file_path: &str) {
        let (header, data) = Self::as_csv_string(output_to_write);

        let _ = csv_data::create_or_append_header_and_data_strings_file(
            file_path,
            &header,
            &data,
        );
    }
}
