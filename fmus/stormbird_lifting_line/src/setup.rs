use super::*;

impl StormbirdLiftingLine {
    /// Function that checks if the parameters file path is set, and if not, sets the default path
    /// to the resource directory of the FMU. 
    pub fn parameters_path(&self) -> PathBuf {
        let parameters_path: PathBuf = if self.parameters_path.is_empty() {
            let mut path = self.fmu_info.resource_path.clone();

            path.push("parameters.json");

            path
        } else {
            PathBuf::from(&self.parameters_path)
        };

        parameters_path
    }

    /// Function that reads the parameters from the parameters file.
    pub fn read_parameters(&mut self) {
        let parameters_path = self.parameters_path();

        let parameters = FmuParameters::from_json_file(&parameters_path);

        match parameters {
            Ok(parameters) => {
                self.parameters = parameters;
            },
            Err(e) => {
                println!("Error reading parameters file: {}", e);
            }
        }
    }

    /// Builds the sail model using the lifting liner setup file
    pub fn build_lifting_line_model(&mut self) {
        let mut setup_path = self.parameters_path();
        setup_path.pop();
        setup_path.push(self.parameters.lifting_line_setup_file_path.clone());

        let stormbird_model_builder =
            SimulationBuilder::new_from_file(&setup_path.to_string_lossy());

        match stormbird_model_builder {
            Ok(builder) => {
                self.stormbird_model = Some(builder.build());
            },
            Err(e) => {
                println!(
                    "Error reading lifting line setup file from path: {}. Error: {}", 
                    &setup_path.to_string_lossy(), 
                    e
                );
            }
        }
    }

    /// Builds filters for the input
    pub fn build_filters(&mut self) {
        if self.parameters.input_moving_average_window_size > 0 {
            self.input_filters = Some(
                InputFilters::new(self.parameters.input_moving_average_window_size)
            );
        }
    }

    /// Builds a wind environment model
    pub fn build_wind_model(&mut self) {
        if !self.parameters.wind_environment_setup_file_path.is_empty() {
            let mut setup_path = self.parameters_path();
            setup_path.pop();
            setup_path.push(self.parameters.wind_environment_setup_file_path.clone());

            let environment = WindEnvironment::from_json_file(&setup_path.to_string_lossy());

            match environment {
                Ok(env) => {
                    self.wind_environment = Some(env);
                },
                Err(e) => {
                    println!(
                        "Error reading wind environment setup file from path: {}. Error: {}", 
                        &self.parameters.wind_environment_setup_file_path, 
                        e
                    );
                }
            }
        } else {
            self.wind_environment = Some(WindEnvironment::default());
        }
    }

    pub fn build_controller(&mut self) {
        if !self.parameters.controller_setup_file_path.is_empty() {
            let mut setup_path = self.parameters_path();
            setup_path.pop();
            setup_path.push(self.parameters.controller_setup_file_path.clone());
            
            let controller_builder = ControllerBuilder::from_json_file(
                &setup_path.to_string_lossy()
            );

            match controller_builder {
                Ok(builder) => {
                    self.controller = Some(builder.build());
                },
                Err(e) => {
                    println!(
                        "Error reading controller setup file from path: {}. Error: {}", 
                        &self.parameters.controller_setup_file_path, 
                        e
                    );
                }
            }
        }
    }

    pub fn build_superstructure_force_model(&mut self) {
        if !self.parameters.superstructure_force_setup_path.is_empty() {
            let mut setup_path = self.parameters_path();
            setup_path.pop();
            setup_path.push(self.parameters.superstructure_force_setup_path.clone());
            
            let superstructure_force_model = BlendermannSuperstructureForces::from_json_file(
                &setup_path.to_string_lossy()
            );

            match superstructure_force_model {
                Ok(model) => {
                    self.superstructure_force_model = Some(model);
                },
                Err(e) => {
                    println!(
                        "Error reading superstructure force setup file from path: {}. Error: {}", 
                        &self.parameters.superstructure_force_setup_path, 
                        e
                    );
                }
            }
        }
    }
}