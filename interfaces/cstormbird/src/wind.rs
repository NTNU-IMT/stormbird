

use stormbird::wind::wind_condition::WindCondition as WindConditionImpl;
use stormbird::wind::wind_condition::velocity_variation::VelocityVariation;
use stormbird::wind::wind_condition::velocity_variation::power_model::PowerModel;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct WindCondition {
    pub velocity: f64,
    pub direction_coming_from: f64,
}

impl From<WindCondition> for WindConditionImpl {
    fn from(c: WindCondition) -> Self {
        WindConditionImpl {
            direction_coming_from: c.direction_coming_from,
            velocity_variation: VelocityVariation::PowerModel(
                PowerModel {
                    reference_velocity: c.velocity, 
                    reference_height: PowerModel::default_reference_height(), 
                    power_factor: PowerModel::default_power_factor()
                }
            ),
            parallel_gust: None,
            perpendicular_gust: None,
            vertical_gust: None
        }
    }
}
