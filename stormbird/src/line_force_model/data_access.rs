// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Implementation block for all the methods that access data from the line force model

use super::*;

impl LineForceModel {
    /// Short hand for querying for the number of wings in the model
    pub fn nr_wings(&self) -> usize {
        self.wing_indices.len()
    }

    /// Short hand for querying for the number of span lines in the model
    pub fn nr_span_lines(&self) -> usize {
        self.span_lines_local.len()
    }

    /// Returns the axis of rotation for the wing at the input index.
    pub fn wing_rotation_axis(&self, wing_index: usize) -> SpatialVector {
        self.span_lines_local[self.wing_indices[wing_index].start].relative_vector()
    }

    pub fn wing_rotation_axis_from_global_index(&self, global_index: usize) -> SpatialVector {
        let wing_index = self.wing_index_from_global(global_index);

        self.wing_rotation_axis(wing_index)
    }

    /// Returns both angle and axis of rotation for the wing at the input index.
    pub fn wing_rotation_data(&self, wing_index: usize) -> (Float, SpatialVector) {
        let axis = self.wing_rotation_axis(wing_index);
        let angle = self.local_wing_angles[wing_index];

        (angle, axis)
    }

    pub fn wing_rotation_data_from_global_index(&self, global_index: usize) -> (Float, SpatialVector) {
        let wing_index = self.wing_index_from_global(global_index);

        self.wing_rotation_data(wing_index)
    }

    pub fn projected_areas(&self) -> Vec<Float> {
        let mut areas = vec![0.0; self.nr_wings()];

        for i in 0..self.nr_span_lines() {
            let wing_index = self.wing_index_from_global(i);

            areas[wing_index] +=
                self.chord_lengths[i] * self.span_lines_local[i].length();
        }

        areas
    }

    /// returns the span length of each wing in the model
    pub fn span_lengths(&self) -> Vec<Float> {
        let mut span_length = vec![0.0; self.nr_wings()];

        for i in 0..self.nr_span_lines() {
            let wing_index = self.wing_index_from_global(i);

            span_length[wing_index] += self.span_lines_local[i].length();
        }

        span_length
    }

    pub fn aspect_ratios(&self) -> Vec<Float> {
        let areas = self.projected_areas();
        let span_lengths = self.span_lengths();

        let mut aspect_ratios = vec![0.0; self.nr_wings()];

        for i in 0..self.nr_wings() {
            if span_lengths[i] > 0.0 {
                aspect_ratios[i] = (span_lengths[i].powi(2)) / areas[i];
            } else {
                aspect_ratios[i] = 0.0;
            }
        }

        aspect_ratios
    }

    /// Integrates the chord length along the span of all wings in the model to return the total
    /// projected area of the wing.
    pub fn total_projected_area(&self) -> Float {
        let mut total_area = 0.0;

        for i in 0..self.nr_span_lines() {
            total_area += self.chord_lengths[i] * self.span_lines_local[i].length();
        }

        total_area
    }

    /// Returns the internal state for the section model belonging to each wing in the model.
    pub fn section_models_internal_state(&self) -> Vec<Float> {
        let mut internal_state: Vec<Float> = vec![0.0; self.nr_wings()];

        for wing_index in 0..self.nr_wings() {
            match self.section_models[wing_index] {
                SectionModel::VaryingFoil(ref foil) => {
                    internal_state[wing_index] = foil.current_internal_state;
                }
                SectionModel::RotatingCylinder(ref cylinder) => {
                    internal_state[wing_index] = cylinder.revolutions_per_second;
                }
                _ => {}
            }
        }

        internal_state
    }

    
}