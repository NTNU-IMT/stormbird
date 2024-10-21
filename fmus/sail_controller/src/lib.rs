
use fmu_from_struct::prelude::*;

use math_utils::spatial_vector::SpatialVector;

use stormbird::controllers::sail_controller::SailController as SailControllerInternal;

use std::fs;

#[derive(Debug, Default, Clone, Fmu)]
#[fmi_version = 2]
pub struct SailController {
    #[parameter]
    pub setup_file_path: String,
    #[input]
    pub velocity_measurements: String,
    #[output]
    pub velocity_measurement_points: String,
    pub local_wing_angles: String,
    pub wind_directions: String,

    controllers: Option<Vec<SailControllerInternal>>,
}

impl FmuFunctions for SailController {
    fn exit_initialization_mode(&mut self) {
        
        let setup_string = fs::read_to_string(&self.setup_file_path).unwrap();

        let controllers: Vec<SailControllerInternal> = serde_json::from_str(&setup_string).unwrap();

        self.controllers = Some(controllers);
    }

    fn do_step(&mut self, _current_time: f64, _time_step: f64) {
        if let Some(controllers) = &self.controllers {
            let velocity_measurements: Vec<SpatialVector<3>> = if !self.velocity_measurements.is_empty() {
                serde_json::from_str(&self.velocity_measurements).unwrap()
            } else {
                vec![SpatialVector([0.0, 0.0, 0.0]); controllers.len()]
            };

            let mut velocity_measurement_points: Vec<SpatialVector<3>> = Vec::new();
            let mut local_wing_angles: Vec<f64> = Vec::new();
            let mut wind_directions: Vec<f64> = Vec::new();

            for (controller, velocity) in controllers.into_iter().zip(velocity_measurements) {
                let controller_result = controller.get_controller_result(velocity);

                velocity_measurement_points.push(controller.measurement_point);
                local_wing_angles.push(controller_result.wing_angle);

                wind_directions.push(controller.wind_direction(velocity));
            }

            self.velocity_measurement_points = serde_json::to_string(&velocity_measurement_points).unwrap();
            self.local_wing_angles = serde_json::to_string(&local_wing_angles).unwrap();
            self.wind_directions = serde_json::to_string(&wind_directions).unwrap();
        }
    }
    
}