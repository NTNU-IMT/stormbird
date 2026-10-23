// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use serde::{Serialize, Deserialize};

use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Simplified result for a single sail
pub struct SingleSailResult {
    /// Total force on the sail
    pub force: SpatialVector,
    /// Total moment on the sail
    pub moment: SpatialVector,
    /// Input power to the sail
    pub input_power: Float
}