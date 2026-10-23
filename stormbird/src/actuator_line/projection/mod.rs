// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use serde::{Serialize, Deserialize};

use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;
use crate::line_force_model::span_line::SpanLine;

/// Gaussian projection function
pub mod gaussian;

use gaussian::Gaussian;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Settings for the projection of the forces from the line elements onto the CFD grid
pub struct ProjectionSettings {
    /// The function used to distribute the forces in space
    #[serde(default)]
    pub projection_function: Gaussian,
    /// If true, the lift forces are realigned to be normal to the local velocity at the control
    /// points
    #[serde(default="ProjectionSettings::default_true")]
    pub realign_sectional_forces: bool,
    /// If true, the lift forces are realigned to be normal to the local velocity in each cell, rather
    /// than at the control points
    #[serde(default)]
    pub realign_to_local_velocity_at_each_cell: bool,
    /// Cells with a projection weight below this value do not get any force
    #[serde(default="ProjectionSettings::default_weight_limit")]
    pub weight_limit: Float,
    /// If true, the sectional drag forces are also projected onto the grid
    #[serde(default)]
    pub project_sectional_drag: bool,
}

impl Default for ProjectionSettings {
    
    fn default() -> Self {
        Self {
            projection_function: Gaussian::default(),
            realign_sectional_forces: true,
            realign_to_local_velocity_at_each_cell: false,
            weight_limit: Self::default_weight_limit(),
            project_sectional_drag: false,
        }
    }
}


impl ProjectionSettings {
    fn default_true() -> bool {true}
    fn default_weight_limit() -> Float {0.001}

    /// Returns the value of the projection function at a point, for the given line element
    pub fn projection_value_at_point(
        &self,
        point: SpatialVector,
        chord_vector: SpatialVector,
        span_line: &SpanLine,
        extended_span_falloff: [bool; 2],
    ) -> Float {
        self.projection_function.projection_value_at_point(
            point, chord_vector, span_line, extended_span_falloff
        )
    }
}