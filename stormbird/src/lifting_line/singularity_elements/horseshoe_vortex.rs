// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)


use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

use crate::line_force_model::LineForceModel;
use crate::line_force_model::span_line::SpanLine;

use super::vortex_line;

#[derive(Clone, Debug)]
pub struct HorseshoeVortex {
    pub bound_vortex: [SpatialVector; 2],
    pub start_trailing_vortex: [SpatialVector; 2],
    pub end_trailing_vortex: [SpatialVector; 2],
    pub viscous_core_length: Float,
}

impl HorseshoeVortex {
    pub fn induced_velocity_with_unit_strength(
        &self,
        ctrl_point: SpatialVector,
    ) -> SpatialVector {
        let mut induced_velocity = SpatialVector::default();

        // Induced velocity from bound vortex
        induced_velocity += vortex_line::induced_velocity_from_line_with_unit_strength(
            &self.bound_vortex,
            ctrl_point,
            self.viscous_core_length,
        );

        // Induced velocity from start trailing vortex
        induced_velocity += vortex_line::induced_velocity_from_line_with_unit_strength(
            &self.start_trailing_vortex,
            ctrl_point,
            self.viscous_core_length,
        );

        // Induced velocity from end trailing vortex
        induced_velocity += vortex_line::induced_velocity_from_line_with_unit_strength(
            &self.end_trailing_vortex,
            ctrl_point,
            self.viscous_core_length,
        );

        induced_velocity
    }

    /// Helper function to create a vector of horseshoe vortices from span lines and wake vectors.
    pub fn vortices_from_line_force_model_and_wake_vectors(
        line_force_model: &LineForceModel,
        wake_vectors: &[SpatialVector],
        viscous_core_length: Float
    ) -> Vec<Self> {
        
        let nr_span_lines = line_force_model.nr_span_lines();
        let nr_wings = line_force_model.wing_indices.len();
        
        let span_lines = &line_force_model.span_lines_global;

        let mut horseshoe_vortices: Vec<Self> = Vec::with_capacity(nr_span_lines);
        
        let mut first_wake_vector_index = 0;
        for wing_index in 0..nr_wings {
            let wing_line_indices = line_force_model.wing_indices[wing_index].clone();
            
            for line_index in wing_line_indices.start..wing_line_indices.end {
                let bound_vortex = [
                    span_lines[line_index].start_point, 
                    span_lines[line_index].end_point
                ];
                
                let start_trailing_vortex = [
                    span_lines[line_index].start_point + wake_vectors[first_wake_vector_index], 
                    span_lines[line_index].start_point
                ];
                
                let end_trailing_vortex = [
                    span_lines[line_index].end_point, 
                    span_lines[line_index].end_point + wake_vectors[first_wake_vector_index + 1]
                ];
                
                let vortex = Self{
                    bound_vortex,
                    start_trailing_vortex,
                    end_trailing_vortex,
                    viscous_core_length,
                };
                
                horseshoe_vortices.push(vortex);
                
                first_wake_vector_index += 1
            }
            
            first_wake_vector_index += 1;
        }

        horseshoe_vortices
    }
    
    /// Helper function to create a vector of horseshoe vortices from span lines and wake vectors.
    /// 
    /// WARNING: should only be used to create horseshoe vortices for a single wing.
    pub fn vortices_for_single_wing_from_span_lines_and_wake_vectors(
        span_lines: &[SpanLine],
        wake_vectors: &[SpatialVector],
        viscous_core_length: Float
    ) -> Vec<Self> {
        let nr_span_lines = span_lines.len();

        let mut horseshoe_vortices: Vec<Self> = Vec::with_capacity(nr_span_lines);

        for i in 0..nr_span_lines {
            let vortex = Self{
                bound_vortex: [span_lines[i].start_point, span_lines[i].end_point],
                start_trailing_vortex: [span_lines[i].start_point + wake_vectors[i], span_lines[i].start_point],
                end_trailing_vortex: [span_lines[i].end_point, span_lines[i].end_point + wake_vectors[i + 1]],
                viscous_core_length,
            };

            horseshoe_vortices.push(vortex);
        }

        horseshoe_vortices
    }
}
