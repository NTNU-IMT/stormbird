// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)


use super::*;

impl StormbirdLiftingLine {
    /// Function that checks if the parameters file path is set, and if not, sets the default path
    /// to the resource directory of the FMU.
    pub fn parameters_path(&self) -> PathBuf {
        if self.parameters_path.is_empty() {
            let mut path = self.fmu_info.resource_path.clone();

            path.push("parameters.json");

            path
        } else {
            PathBuf::from(&self.parameters_path)
        }
    }

    /// Reads the parameters file and builds all the parts of the model.
    pub fn initialize(&mut self) -> Result<(), FmuError> {
        let parameters_path = self.parameters_path();

        let parameters = FmuParameters::from_json_file(&parameters_path).map_err(|error| {
            FmuError::new(
                error.code(),
                format!(
                    "Could not read the parameters file {}: {}",
                    parameters_path.to_string_lossy(),
                    error
                )
            )
        })?;

        let sail_model = parameters.sail_model.build()?;

        let nr_sails = sail_model.get_number_of_sails();

        if nr_sails > MAX_NR_SAILS {
            return Err(FmuError::new(
                error_codes::TOO_MANY_SAILS,
                format!(
                    "The sail model has {} sails, but the FMU only supports up to {} sails",
                    nr_sails, MAX_NR_SAILS
                )
            ));
        }

        if parameters.input_moving_average_window_size > 0 {
            self.input_filters = Some(
                InputFilters::new(parameters.input_moving_average_window_size)
            );
        }

        if self.time_model_scale > 0.0 {
            self.time_model_scaling = Some(
                ModelScaling {
                    scale: self.time_model_scale
                }
            );
        }

        self.parameters = Some(parameters);
        self.sail_model = Some(sail_model);

        Ok(())
    }
}
