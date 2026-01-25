

use stormbird::wind::wind_condition::WindCondition as WindConditionImpl;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct WindCondition {
    pub velocity: f64,
    pub direction_coming_from: f64,
}

impl From<WindCondition> for WindConditionImpl {
    fn from(c: WindCondition) -> Self {
        WindConditionImpl {
            velocity: c.velocity,
            direction_coming_from: c.direction_coming_from
        }
    }
}