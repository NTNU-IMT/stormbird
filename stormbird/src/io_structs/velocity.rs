// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Functionality to represent the velocity in simulations due to both the freestream velocity and 
//! the motion of the wings.

use crate::vec3::Vec3;

#[derive(Debug, Clone, Copy, Default)]
/// Structure to store and acess the freestream velocity and the motion velocity of the wing.
pub struct VelocityInput {
    /// Freestream velocity, measured in m/s
    pub freestream: Vec3,
    /// TRanslation velocity of the wing(s), measured in m/s
    pub translation: Vec3,
    /// Rotation velocity of the wing(s), measured in rad/s
    pub rotation: Vec3,
}

impl VelocityInput {
    /// Returns the freestream velocity.
    pub fn freestream(&self) -> Vec3 {
        self.freestream
    }

    /// Calculates the felt velocity at the given points. The motion velocity is assumed to be 
    /// negative, meaning that the internal values represent the velocity of the wing.
    pub fn felt_velocity_at_points(&self, points: &[Vec3]) -> Vec<Vec3> {
        points.iter().map(|point| {
            self.freestream - self.translation - self.rotation.cross(*point)
        }).collect()
    }
}


#[derive(Debug, Clone, Copy, Default)]
/// Structure to store input to a simulation
pub struct InputState {
    /// Freestream velocity, measured in m/s
    pub freestream_velocity: Vec3,
    /// Translation of the wing(s), measured in m
    pub translation: Vec3,
    /// Rotation of the wing(s), measured in rad
    pub rotation: Vec3,
}

#[derive(Debug, Clone, Copy)]
/// A structure responsible for calculating the velocity based on the input state of the wings, 
/// using finite difference and the history of the input states.
pub struct VelocityCalculator {
    /// The history of the input states
    pub state_history: [InputState; 2],
}

impl Default for VelocityCalculator {
    fn default() -> Self {
        Self {
            state_history: [InputState::default(); 2],
        }
    }
}

impl VelocityCalculator {
    pub fn get_velocity_input(&mut self, input_state: InputState, time_step: f64) -> VelocityInput {
        let rotation_velocity = (
            3.0 * input_state.rotation - 
            4.0 * self.state_history[1].rotation + 
            self.state_history[0].rotation
        ) / (2.0 * time_step);

        let translation_velocity = (
            3.0 * input_state.translation - 
            4.0 * self.state_history[1].translation + 
            self.state_history[0].translation
        ) / (2.0 * time_step);

        self.state_history[0] = self.state_history[1];
        self.state_history[1] = input_state;

        VelocityInput {
            freestream: input_state.freestream_velocity,
            translation: translation_velocity,
            rotation: rotation_velocity,
        }
    }
}
