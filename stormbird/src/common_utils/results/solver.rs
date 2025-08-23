
use stormath::{
    spatial_vector::SpatialVector,
    type_aliases::Float,
};

#[derive(Debug, Clone)]
/// Results from a lifting line solver, which will be further used to generate SimulationResults
pub struct SolverResult {
    pub circulation_strength: Vec<Float>,
    pub ctrl_point_velocity: Vec<SpatialVector>,
    pub iterations: usize,
    pub residual: Float,
}