// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)


use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

use crate::line_force_model::LineForceModel;
use crate::line_force_model::span_line::SpanLine;

use super::vortex_line::VortexLine;

const ELBOW_VORTEX_LENGTH_FACTOR: Float = 0.15; // TODO: evaluate the need to have this as a parameter

#[derive(Clone, Debug)]
/// A horseshoe vortex is the classical building block to represent wings. It consists of a bound
/// vortex and two free vortices, all with the same strength, that can be used ot calculate the 
/// lift-induced velocities from a section of the wing.
pub struct HorseshoeVortex {
    /// A collection of vortex lines that make up the horseshoe vortex. The direction of the lines 
    /// matter for the values of the lift-induced velocities. As such, it is important that this one
    /// is created in the right way. The logic for creating this is found in the method `Self::new`.
    /// 
    /// There array stores five collections of spatial vectors, that later is used to represent 
    /// vortex lines. The intended meaning of them is as follows:
    /// 1) The first line goes from the end of the first free vortex to the "elbow vortex" (see next
    /// vortex definition)
    /// 2) The second line is the first "elbow vortex". It connect the bound vortex with the first 
    /// free stream vortex in a direction normal to the bound vortex.However, the direction is from 
    /// point closest to the bound vortex in the first free vortex, to the first point of the bound 
    /// vortex to maintain the right direction.
    /// 3) The third line is the bound vortex itself, going from the start of the span line to the
    /// end of the span line.
    /// 4) The fourth line is the elbow vortex, connecting the end of the bound vortex to the second
    /// free vortex in a direction normal to the bound vortex.
    /// 5) The fifth and last line is the second free vortex, going from the end of the second elbow
    /// vortex in the direction of the free stream
    vortex_lines: [VortexLine; 5],
    /// The viscous core length used to limit the induced velocity close to the vortices. The main 
    /// point is to avoid singularities.
    viscous_core_length: Float,
}

impl HorseshoeVortex {
    /// Creates a new horseshoe vortex from the supplied span line, wake vectors and viscous core 
    /// length
    pub fn new(
        span_line: &SpanLine,
        wake_vectors: &[SpatialVector; 2],
        elbow_length: Float,
        viscous_core_length: Float
    ) -> Self {    
        let bound_vortex = VortexLine::new([
            span_line.start_point, 
            span_line.end_point
        ]);

        let span_direction = span_line.relative_vector().normalize();
        let bound_normal_direction = wake_vectors[0].project_on_plane(span_direction);

        let elbow_vector = elbow_length * bound_normal_direction;

        let first_elbow_vortex = VortexLine::new([
            span_line.start_point + elbow_vector,
            span_line.start_point
        ]);
        
        let first_trailing_vortex =  VortexLine::new([
            first_elbow_vortex.points[0] + wake_vectors[0], 
            first_elbow_vortex.points[0]
        ]);

        let second_elbow_vortex = VortexLine::new([
            span_line.end_point, 
            span_line.end_point + elbow_vector
        ]);

        // From the end of the bound vector to the end of the last free vortex
        let second_trailing_vortex = VortexLine::new([
            second_elbow_vortex.points[1], 
            second_elbow_vortex.points[1] + wake_vectors[1]
        ]);

        Self {
            vortex_lines: [
                first_trailing_vortex,
                first_elbow_vortex,
                bound_vortex,
                second_elbow_vortex,
                second_trailing_vortex
            ],
            viscous_core_length
        }
    }
    /// Calculates the induced velocity from all components of the horseshoe vortex, assuming the 
    /// strength is unity.
    pub fn induced_velocity_with_unit_strength(&self, ctrl_point: SpatialVector) -> SpatialVector {
        self.vortex_lines.iter().map(
            |line| {
                line.induced_velocity_from_line_with_unit_strength(
                    ctrl_point,
                    self.viscous_core_length,
                )
            }
        ).sum::<SpatialVector>()
    }

    /// Helper function to create a vector of horseshoe vortices from span lines and wake vectors.
    /// 
    /// Arguments
    /// - `line_force_model`: a pointer to the line force model the wake is intended for
    /// - `wake_vectors`: the vectors that defines the direction and length of each of the free 
    /// vortices. It will typically be based on the direction of the velocity at the span points of
    /// the lifting line, but logic for determining this is delegated to outside this function
    /// - `viscous_core_length`: The viscous core length to give to each horseshoe vortex structure
    pub fn vortices_from_line_force_model_and_wake_vectors(
        line_force_model: &LineForceModel,
        wake_vectors: &[SpatialVector],
        viscous_core_length: Float
    ) -> Vec<Self> {
        
        let nr_span_lines = line_force_model.nr_span_lines();
        let nr_wings = line_force_model.wing_indices.len();
        
        let span_lines = &line_force_model.span_lines_global;
        let chord_lengths = &line_force_model.chord_lengths;

        let mut horseshoe_vortices: Vec<Self> = Vec::with_capacity(nr_span_lines);
        
        let mut first_wake_vector_index = 0;
        for wing_index in 0..nr_wings {            
            let wing_line_indices = line_force_model.wing_indices[wing_index].clone();
            
            for line_index in wing_line_indices.start..wing_line_indices.end {
                let span_line = &span_lines[line_index];
                let local_wake_vectors = [
                    wake_vectors[first_wake_vector_index],
                    wake_vectors[first_wake_vector_index + 1]
                ];

                let elbow_length = chord_lengths[line_index] * ELBOW_VORTEX_LENGTH_FACTOR;

                horseshoe_vortices.push(
                    Self::new(
                        &span_line,
                        &local_wake_vectors,
                        elbow_length,
                        viscous_core_length
                    )
                );
                
                first_wake_vector_index += 1
            }
            
            first_wake_vector_index += 1;
        }

        horseshoe_vortices
    }

    
    
    /// Helper function to create a vector of horseshoe vortices for a single wing only, based on 
    /// the supplied span lines and wake vectors. 
    pub fn vortices_for_single_wing_from_span_lines_and_wake_vectors(
        span_lines: &[SpanLine],
        chord_lengths: &[Float],
        wake_vectors: &[SpatialVector],
        viscous_core_length: Float
    ) -> Vec<Self> {
        span_lines.iter().enumerate().map(
            |(line_index, span_line)| {
                let local_wake_vectors = [
                    wake_vectors[line_index],
                    wake_vectors[line_index + 1]
                ];

                let elbow_length = chord_lengths[line_index] * ELBOW_VORTEX_LENGTH_FACTOR;

                Self::new(
                    span_line,
                    &local_wake_vectors,
                    elbow_length,
                    viscous_core_length
                )
            }
        ).collect()
    }
}
