use crate::vec3::Vec3;
use crate::line_force_model::LineForceModel;

use crate::math_utils::finite_difference;

#[derive(Debug, Clone, Default)]
/// Structure to store necesseray data to calculate the forces on a line force model
/// 
/// Each vector in this struct represents values at control points for the line force model.
pub struct ForceInput {
    /// Velocity at the control points
    pub velocity: Vec<Vec3>,
    /// Acceleration at the control points. Mainly used for calculating added mass forces
    pub acceleration: Vec<Vec3>,
    /// How fast the chord vector is rotating. Mainly used to calculate additinal lift due to this
    /// rotation
    pub chord_rotation_velocity: Vec<Vec3>,
}

#[derive(Debug, Clone, Default)]
/// Structure used to estimate the force input for a given time step, based on the history of the
/// motion of the wings.
pub struct ForceInputCalculator {
    /// Previous positions of the control points
    ctrl_points_history: [Vec<Vec3>; 2],
    /// Previous values for the chord vector
    chord_vector_history: [Vec<Vec3>; 2],
}

impl ForceInputCalculator {
    pub fn new(line_force_model: &LineForceModel) -> Self {
        let ctrl_points = line_force_model.ctrl_points();
        let chord_vectors = line_force_model.chord_vectors();
        
        Self {
            ctrl_points_history: [ctrl_points.clone(), ctrl_points.clone()],
            chord_vector_history: [chord_vectors.clone(), chord_vectors.clone()],
        }
    }

    /// Calculates the force input for a given time step.
    /// 
    /// The velocity and acceleration at each control point is estimated from finitie difference 
    /// calcaultion of the stored values of ctrl points and chord vectors.
    pub fn get_force_input(
        &mut self, 
        line_force_model: &LineForceModel, 
        freestream_velocity: &[Vec3], 
        time_step: f64
    ) -> ForceInput {
        if line_force_model.nr_span_lines() != self.ctrl_points_history[0].len() {
            panic!(
                "The number of span lines in the line force model does not match the number of span lines in the force input calculator"
            );
        }

        let current_ctrl_points = line_force_model.ctrl_points();
        let current_chord_vectors = line_force_model.chord_vectors();

        let mut velocity: Vec<Vec3> = Vec::with_capacity(line_force_model.nr_span_lines());
        
        for i in 0..line_force_model.nr_span_lines() {
            let postion_hisotry = [
                self.ctrl_points_history[0][i],
                self.ctrl_points_history[1][i],
                current_ctrl_points[i]
            ];

            let motion_velocity = finite_difference::first_derivative_second_order_backward(
                &postion_hisotry, time_step
            );

            velocity.push(freestream_velocity[i] - motion_velocity);
        }

        let acceleration = vec![Vec3::default(); line_force_model.nr_span_lines()];
        let chord_rotation_velocity = vec![Vec3::default(); line_force_model.nr_span_lines()];

        for i in 0..line_force_model.nr_span_lines() {
            self.ctrl_points_history[1][i] = self.ctrl_points_history[0][i];
            self.ctrl_points_history[0][i] = current_ctrl_points[i];

            self.chord_vector_history[1][i] = self.chord_vector_history[0][i];
            self.chord_vector_history[0][i] = current_chord_vectors[i];
        }

        ForceInput {
            velocity,
            acceleration,
            chord_rotation_velocity,
        }
    }
}