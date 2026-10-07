pub mod context;
pub mod utils;

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
/// Where a solver is executed
pub enum ComputePlatform {
    #[default]
    CPU,
    GPU
}

impl ComputePlatform {
    pub fn is_gpu(&self) -> bool {
        *self == ComputePlatform::GPU
    }
}
