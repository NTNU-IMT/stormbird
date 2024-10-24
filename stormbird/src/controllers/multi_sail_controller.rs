use super::pid_controller::PIDController;

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct MultiSailController {
    pub pid_controllers: Vec<PIDController>
}

impl MultiSailController {
    pub fn compute_new_wing_angles(&mut self, time_step: f64, angle_measurments: &[f64]) -> Vec<f64> {
        (0..self.pid_controllers.len()).map(
            |i| {
                self.pid_controllers[i].do_step(time_step, angle_measurments[i])
            }
        ).collect()
    }
}
