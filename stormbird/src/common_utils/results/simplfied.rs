// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use serde::{Serialize, Deserialize};

use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SingleSailResult {
    pub force: SpatialVector,
    pub moment: SpatialVector,
    pub input_power: Float
}