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
    pub fn wing_rotation_axis(&self, wing_index: usize) -> SpatialVector<3> {
        self.span_lines_local[self.wing_indices[wing_index].start].relative_vector()
    }

    pub fn wing_rotation_axis_from_global(&self, global_index: usize) -> SpatialVector<3> {
        let wing_index = self.wing_index_from_global(global_index);

        self.wing_rotation_axis(wing_index)
    }

    /// Returns both angle and axis of rotation for the wing at the input index.
    pub fn wing_rotation_data(&self, wing_index: usize) -> (f64, SpatialVector<3>) {
        let axis = self.wing_rotation_axis(wing_index);
        let angle = self.local_wing_angles[wing_index];

        (angle, axis)
    }

    pub fn wing_rotation_data_from_global(&self, global_index: usize) -> (f64, SpatialVector<3>) {
        let wing_index = self.wing_index_from_global(global_index);

        self.wing_rotation_data(wing_index)
    }

    pub fn global_chord_vector_at_index(&self, index: usize) -> SpatialVector<3> {
        self.rigid_body_motion.transform_vector(self.local_chord_vector_at_index(index))
    }

    /// Returns the chord vectors in global coordinates.
    pub fn local_chord_vectors(&self) -> Vec<SpatialVector<3>> {
        self.chord_vectors_local
            .iter()
            .enumerate()
            .map(|(global_index, chord_vector)| {
                let (angle, axis) = self.wing_rotation_data_from_global(global_index);

                chord_vector.rotate_around_axis(angle, axis)
            })
            .collect()
    }

    pub fn global_chord_vectors(&self) -> Vec<SpatialVector<3>> {
        let local_chord_vectors = self.local_chord_vectors();

        local_chord_vectors
            .iter()
            .map(|chord_vector| self.rigid_body_motion.transform_vector(*chord_vector))
            .collect()
    }

    /// Returns the control points of each line element. This is calculated as the midpoint of each
    /// span line
    pub fn ctrl_points(&self) -> Vec<SpatialVector<3>> {
        let span_lines = self.span_lines();

        span_lines
            .iter()
            .map(|line| line.ctrl_point())
            .collect()
    }

    /// Returns the control points of each line element in local coordinates. This is calculated as
    /// the midpoint of each span line
    pub fn ctrl_points_local(&self) -> Vec<SpatialVector<3>> {
        self.span_lines_local
            .iter()
            .map(|line| line.ctrl_point())
            .collect()
    }

    /// Returns the points making up the line geometry of the wings as a vector of spatial vectors,
    /// as opposed to a vector of span lines.
    pub fn span_points(&self) -> Vec<SpatialVector<3>> {
        let span_lines = self.span_lines();
        let mut span_points: Vec<SpatialVector<3>> = Vec::new();

        for wing_index in 0..self.wing_indices.len() {
            for i in self.wing_indices[wing_index].clone() {
                span_points.push(span_lines[i].start_point);
            }

            let last_index = self.wing_indices[wing_index].clone().last().unwrap();

            span_points.push(span_lines[last_index].end_point);
        }

        span_points
    }

    #[inline(always)]
    pub fn span_line_at_index(&self, index: usize) -> SpanLine {
        let mut span_line = self.span_lines_local[index].clone();

        span_line.start_point = self.rigid_body_motion.transform_point(span_line.start_point);
        span_line.end_point = self.rigid_body_motion.transform_point(span_line.end_point);

        span_line
    }

    /// Returns the span lines in global coordinates.
    pub fn span_lines(&self) -> Vec<SpanLine> {
        (0..self.nr_span_lines())
            .map(|i| self.span_line_at_index(i))
            .collect()
    }

    pub fn local_chord_vector_at_index(&self, index: usize) -> SpatialVector<3> {
        let (angle, axis) = self.wing_rotation_data_from_global(index);

        self.chord_vectors_local[index].rotate_around_axis(angle, axis)
    }

    pub fn projected_areas(&self) -> Vec<f64> {
        let mut areas = vec![0.0; self.nr_wings()];

        for i in 0..self.nr_span_lines() {
            let wing_index = self.wing_index_from_global(i);

            areas[wing_index] +=
                self.chord_vectors_local[i].length() * self.span_lines_local[i].length();
        }

        areas
    }

    /// returns the span length of each wing in the model
    pub fn span_lengths(&self) -> Vec<f64> {
        let mut span_length = vec![0.0; self.nr_wings()];

        for i in 0..self.nr_span_lines() {
            let wing_index = self.wing_index_from_global(i);

            span_length[wing_index] += self.span_lines_local[i].length();
        }

        span_length
    }

    pub fn aspect_ratios(&self) -> Vec<f64> {
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
    pub fn total_projected_area(&self) -> f64 {
        let mut total_area = 0.0;

        for i in 0..self.nr_span_lines() {
            total_area += self.chord_vectors_local[i].length() * self.span_lines_local[i].length();
        }

        total_area
    }

    pub fn section_models_internal_state(&self) -> Vec<f64> {
        let mut internal_state: Vec<f64> = vec![0.0; self.nr_wings()];

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

    pub fn model_state(&self) -> LineForceModelState {
        LineForceModelState {
            local_wing_angles: self.local_wing_angles.clone(),
            section_models_internal_state: self.section_models_internal_state(),
        }
    }

    pub fn span_distance_in_local_coordinates(&self) -> Vec<f64> {
        let mut span_distance: Vec<f64> = Vec::new();

        for wing_index in 0..self.wing_indices.len() {
            let start_point =
                self.span_lines_local[self.wing_indices[wing_index].start].start_point;

            let mut previous_point = start_point;
            let mut previous_distance = 0.0;

            let mut current_wing_span_distance: Vec<f64> = Vec::new();

            for i in self.wing_indices[wing_index].clone() {
                let line = &self.span_lines_local[i];

                let increase_in_distance = line.ctrl_point().distance(previous_point);
                previous_point = line.ctrl_point();

                current_wing_span_distance.push(previous_distance + increase_in_distance);

                previous_distance += increase_in_distance;
            }

            let end_point = self.span_lines_local
                [self.wing_indices[wing_index].clone().last().unwrap()]
            .end_point;

            let total_distance =
                current_wing_span_distance.last().unwrap() + end_point.distance(previous_point);

            for i in 0..self.wing_indices[wing_index].end - self.wing_indices[wing_index].start {
                span_distance.push(current_wing_span_distance[i] - 0.5 * total_distance);
            }
        }

        span_distance
    }

    /// Calculates the relative distance from the center off each wing for each control point.
    /// The absolute values are divided with the span of each wing. In other words, the
    /// return value will vary between -0.5 and 0.5, where 0 is the center of the wing.
    pub fn relative_span_distance(&self) -> Vec<f64> {
        let mut relative_span_distance: Vec<f64> = Vec::new();

        for wing_index in 0..self.wing_indices.len() {
            let start_point =
                self.span_lines_local[self.wing_indices[wing_index].start].start_point;

            let mut previous_point = start_point;
            let mut previous_distance = 0.0;

            let mut current_wing_span_distance: Vec<f64> = Vec::new();

            for i in self.wing_indices[wing_index].clone() {
                let line = &self.span_lines_local[i];

                let increase_in_distance = line.ctrl_point().distance(previous_point);
                previous_point = line.ctrl_point();

                current_wing_span_distance.push(previous_distance + increase_in_distance);

                previous_distance += increase_in_distance;
            }

            let end_point = self.span_lines_local
                [self.wing_indices[wing_index].clone().last().unwrap()]
            .end_point;

            let total_distance =
                current_wing_span_distance.last().unwrap() + end_point.distance(previous_point);

            for i in 0..self.wing_indices[wing_index].end - self.wing_indices[wing_index].start {
                relative_span_distance.push(current_wing_span_distance[i] / total_distance - 0.5);
            }
        }

        relative_span_distance
    }

    /// Returns the effective non-dimensional span distance values for each control point.
    pub fn effective_relative_span_distance(&self) -> Vec<f64> {
        let relative_span_distance = self.relative_span_distance();

        relative_span_distance.iter().enumerate().map(
            |(index, value)| {
                let wing_index = self.wing_index_from_global(index);
                    match self.non_zero_circulation_at_ends[wing_index] {
                        [true, true] => *value, // TODO: consider if this case should behave differently. Not clear how it should be handled....
                        [true, false] => (value + 0.5) / 2.0,
                        [false, true] => (value - 0.5) / 2.0,
                        [false, false] => *value
                    }
            }
        ).collect()
    }


}