
use stormath::{
    spatial_vector::SpatialVector,
    type_aliases::Float,
};

#[derive(Debug, Clone)]
/// Results from a lifting line solver, which will be further used to generate SimulationResults
pub struct SolverResult {
    pub input_ctrl_point_velocity: Vec<SpatialVector>,
    pub circulation_strength: Vec<Float>,
    pub output_ctrl_point_velocity: Vec<SpatialVector>,
    pub iterations: usize,
    pub residual: Float,
}