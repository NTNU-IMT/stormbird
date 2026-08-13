
use serde::{Serialize, Deserialize};

use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridBuilder {
    pub start_point: SpatialVector,
    pub end_point: SpatialVector,
    pub cells_per_representative_length: [usize; 3],
    pub representative_length: Option<Float>
}