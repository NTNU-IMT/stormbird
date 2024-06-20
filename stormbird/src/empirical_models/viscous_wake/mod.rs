use crate::vec3::Vec3;

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone)]
pub enum WakeState {
    InWake(WakeCoordinates),
    NotInWake,
}

#[derive(Debug, Clone)]
pub struct WakeCoordinates {
    pub velocity: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
/// Model for a viscous wake
pub struct ViscousWake {
    pub start_point: Vec3,
    pub width_vector: Vec3,
    pub height_vector: Vec3,
    pub drag_coefficient: f64, 
}

impl ViscousWake {
    pub fn normal(&self) -> Vec3 {
        self.width_vector.cross(self.height_vector).normalize()
    }

    pub fn compute_wake_state_of_point(&self, point: Vec3, velocity: Vec3) -> WakeState {
        let relative_point = point - self.start_point;

        let effective_width = self.width.project_on_plane(velocity);
        let effective_height = self.height.project_on_plane(velocity);

        let height_coordinate = relative_point.dot(effective_height);
        let width_coordinate = relative_point.dot(effective_width);
        
    }
}