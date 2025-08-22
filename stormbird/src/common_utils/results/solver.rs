
use stormath::spatial_vector::SpatialVector;

#[derive(Debug, Clone)]
/// Results from a lifting line solver, which will be further used to generate SimulationResults
pub struct SolverResult {
    pub circulation_strength: Vec<f64>,
    pub ctrl_point_velocity: Vec<SpatialVector>,
    pub iterations: usize,
    pub residual: f64,
}