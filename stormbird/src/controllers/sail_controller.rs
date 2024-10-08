
use math_utils::spatial_vector::SpatialVector;
use math_utils::interpolation::linear_interpolation;


use serde::{Serialize, Deserialize};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct SailControllerResult {
    pub wing_angle: f64,
    pub internal_state: f64,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct SailController {
    pub reference_direction: SpatialVector<3>,
    pub measurement_point: SpatialVector<3>,
    pub rotation_axis: SpatialVector<3>,
    #[serde(default)]
    pub use_degrees: bool,
    pub wind_direction_data: Vec<f64>,
    #[serde(default)]
    pub wing_angles_data: Vec<f64>,
    #[serde(default)]
    pub internal_state_data: Vec<f64>,
}

impl SailController {
    pub fn get_controller_result(&self, wind_velocity: SpatialVector<3>) -> SailControllerResult {    
        let wind_direction = self.wind_direction(wind_velocity);

        let wing_angle = if self.wing_angles_data.len() > 0 {
            linear_interpolation(
                wind_direction, 
                &self.wind_direction_data, 
                &self.wing_angles_data
            )
        } else {
            0.0
        };

        let internal_state = if self.internal_state_data.len() > 0 {
            linear_interpolation(
                wind_direction, 
                &self.wind_direction_data, 
                &self.internal_state_data
            )
        } else {
            0.0
        };

        SailControllerResult {
            wing_angle,
            internal_state,
        }
    }

    pub fn wind_direction(&self, wind_velocity: SpatialVector<3>) -> f64 {
        let mut wind_direction = self.reference_direction.signed_angle_between(
            wind_velocity, 
            self.rotation_axis
        );

        if self.use_degrees {
            wind_direction = wind_direction.to_degrees();
        }

        wind_direction
    }

}