use serde::{Deserialize, Serialize};

use crate::io_utils::csv_data;
use stormath::type_aliases::Float;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ControllerOutput {
    pub local_wing_angles: Option<Vec<Float>>,
    pub section_models_internal_state: Option<Vec<Float>>,
}

impl ControllerOutput {
    pub fn as_csv_string(&self) -> (String, String) {
        let mut header = String::new();
        let mut data = String::new();

        if let Some(ref angles) = self.local_wing_angles {
            for (i, angle) in angles.iter().enumerate() {
                if i > 0 {
                    header.push(',');
                    data.push(',');
                }

                header.push_str(&format!("local_wing_angle_{}", i));
                data.push_str(&format!("{:.6}", angle));
            }
        }

        if let Some(ref states) = self.section_models_internal_state {
            for (i, state) in states.iter().enumerate() {
                if i > 0 {
                    header.push(',');
                    data.push(',');
                }

                header.push_str(&format!("section_model_internal_state_{}", i));
                data.push_str(&format!("{:.6}", state));
            }
        }

        (header, data)
    }

    pub fn write_to_csv_file(&self, file_path: &str) {
        let (header, data) = self.as_csv_string();

        let _ = csv_data::create_or_append_header_and_data_strings_file(
            file_path,
            &header,
            &data,
        );
    }
}