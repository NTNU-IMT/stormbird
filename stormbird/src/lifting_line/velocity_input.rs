use serde::{Serialize, Deserialize};

use crate::line_force_model::LineForceModel;
use crate::vec3::Vec3;

#[derive(Serialize, Deserialize, Debug, Clone)]
/// Enum used to handle the freestream velocity input to a lifting line simulation.
pub enum VelocityInput {
    /// Variant representing constant freestream velocity at all control points
    Uniform(Vec3),
    /// Variant representing velocity that can have different values at each control point
    Varying(Vec<Vec3>)
}

impl VelocityInput {
    /// Returns the freestream
    pub fn get_velocity_at_ctrl_points(&self, line_force_model: &LineForceModel) -> Vec<Vec3> {
        match self {
            VelocityInput::Uniform(v) => vec![v.clone(); line_force_model.ctrl_points().len()],
            VelocityInput::Varying(v) => {
                assert!(
                    v.len() == line_force_model.nr_span_lines(), 
                    "The number of velocity vectors must match the number of span lines in the line force model."
                );

                v.clone()
            }
        }
    }
}