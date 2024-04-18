use crate::vec3::Vec3;

use super::freestream::Freestream;

#[derive(Debug, Clone, Copy)]
/// Structure to store input to a simulation
pub struct InputState {
    /// Freestream velocity, measured in m/s
    pub freestream: Freestream,
    /// Translation of the wing(s), measured in m
    pub translation: Vec3,
    /// Rotation of the wing(s), measured in rad
    pub rotation: Vec3,
}