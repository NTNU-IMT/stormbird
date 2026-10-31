// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)


//! Implementation block for all the methods that update the state of the line force model


use super::*;

impl LineForceModel {
    /// Updates the global data from the current rigid body transformation and local wing angles.
    pub fn update_global_data_representations(&mut self) {
        self.update_chord_vectors();
        self.update_global_span_lines();
        self.update_global_span_points();
        self.update_ctrl_points();
    }

    /// Updates all values that are calculated from the input geometry. Called after the model is
    /// created.
    pub fn update_calculated_values_after_create(&mut self) {
        self.update_global_data_representations();
        self.update_ctrl_point_spanwise_distance();
    }

    /// Updates the span lines in the global coordinate system, based on the rigid body motion
    pub fn update_global_span_lines(&mut self) {
        if self.span_lines_global.len() != self.span_lines_local.len() {
            self.span_lines_global = self.span_lines_local.clone();
        }

        for i in 0..self.span_lines_local.len() {
            self.span_lines_global[i].start_point = self.rigid_body_motion.transform_point(
                self.span_lines_local[i].start_point
            );

            self.span_lines_global[i].end_point = self.rigid_body_motion.transform_point(
                self.span_lines_local[i].end_point
            );
        }
    }

    /// Updates the span points in the global coordinate system, based on the global span lines
    pub fn update_global_span_points(&mut self) {
        let mut span_points: Vec<SpatialVector> = Vec::new();

        for wing_index in 0..self.wing_indices.len() {
            for i in self.wing_indices[wing_index].clone() {
                span_points.push(self.span_lines_global[i].start_point);
            }

            let last_index = self.wing_indices[wing_index].clone().last().unwrap();

            span_points.push(self.span_lines_global[last_index].end_point);
        }

        self.span_points_global = span_points;
    }

    /// Updates the local and global chord vectors, based on the local wing angles and the rigid body
    /// motion
    pub fn update_chord_vectors(&mut self) {
        if self.chord_vectors_local.len() != self.chord_vectors_local_not_rotated.len() {
            self.chord_vectors_local = self.chord_vectors_local_not_rotated.clone();
        }

        if self.chord_vectors_global.len() != self.chord_vectors_local.len() {
            self.chord_vectors_global = self.chord_vectors_local.clone();
        }

        for i in 0..self.chord_vectors_local_not_rotated.len() {
            let (angle, axis) = self.wing_rotation_data_from_global_index(i);

            self.chord_vectors_local[i] = self.chord_vectors_local_not_rotated[i]
                .rotate_around_axis(angle, axis);

            self.chord_vectors_global[i] = self.rigid_body_motion.transform_vector(
                self.chord_vectors_local[i]
            );
        }

        self.chord_vectors_global_at_span_points = self.span_point_values_from_ctrl_point_values(
            &self.chord_vectors_global, false
        );
    }

    /// Updates the control points in the global coordinate system, based on the global span lines
    pub fn update_ctrl_points(&mut self) {
        if self.ctrl_points_global.len() != self.span_lines_global.len() {
            self.ctrl_points_global = vec![SpatialVector::from([0.0, 0.0, 0.0]); self.span_lines_global.len()];
        }

        for i in 0..self.span_lines_global.len() {
            self.ctrl_points_global[i] = self.span_lines_global[i].ctrl_point();
        }
    }

    /// Updates the distance along the span from the center of each wing to each control point, both
    /// in dimensional and non-dimensional form
    pub fn update_ctrl_point_spanwise_distance(&mut self) {
        self.ctrl_point_spanwise_distance = Vec::with_capacity(self.span_lines_local.len());
        self.ctrl_point_spanwise_distance_non_dimensional = Vec::with_capacity(self.span_lines_local.len());

        for wing_index in 0..self.wing_indices.len() {
            let start_point =
                self.span_lines_local[self.wing_indices[wing_index].start].start_point;

            let mut previous_point = start_point;
            let mut previous_distance = 0.0;

            let mut current_wing_span_distance: Vec<Float> = Vec::new();

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

            for &span_distance in &current_wing_span_distance {
                let relative_distance = span_distance / total_distance;

                self.ctrl_point_spanwise_distance.push(span_distance - 0.5 * total_distance);
                self.ctrl_point_spanwise_distance_non_dimensional.push(relative_distance - 0.5);
            }
        }

        self.ctrl_point_spanwise_distance_circulation_model = 
            self.ctrl_point_spanwise_distance_non_dimensional.iter().enumerate().map(
                |(index, value)| {
                    let wing_index = self.wing_index_from_global(index);
                        match self.non_zero_circulation_at_ends[wing_index] {
                            [true, true] => *value, // TODO: consider if this case should behave differently. Not clear how it should be handled....
                            [true, false] => (value + 0.5) / 2.0,
                            [false, true] => (value - 0.5) / 2.0,
                            [false, false] => *value
                        }
                }
            ).collect();
    }

    /// Sets a constant internal state for each wing
    pub fn set_internal_state(&mut self, internal_state: &[Float]) {
        for (index, value) in internal_state.iter().enumerate() {
            self.internal_state[index] = WingInternalState::Constant(*value);
        }
    }

    /// Sets the internal state of a single wing, which can be constant or vary along the span
    pub fn set_wing_internal_state(&mut self, wing_index: usize, internal_state: WingInternalState) {
        self.internal_state[wing_index] = internal_state;
    }

    /// Sets an internal state that varies along the span for a single wing
    pub fn set_internal_state_span_variation(&mut self, wing_index: usize, span_variation: SpanVariation) {
        self.internal_state[wing_index] = WingInternalState::SpanVarying(span_variation);
    }

    /// Sets the local wing angles and internal states from the output of a controller
    pub fn set_controller_output(&mut self, controller_output: &ControllerOutput) {                
        self.set_local_wing_angles(&controller_output.local_wing_angles);
        self.set_internal_state(&controller_output.internal_states);
    }

    /// Sets the local wing angles, and updates the global geometry
    pub fn set_local_wing_angles(&mut self, local_wing_angles: &[Float]) {
        for (index, angle) in local_wing_angles.iter().enumerate() {
            self.local_wing_angles[index] = *angle;
        }

        self.update_global_data_representations();
    }

    /// Resets the local wing angles to zero.
    pub fn reset_local_wing_angles(&mut self) {
        for angle in self.local_wing_angles.iter_mut() {
            *angle = 0.0;
        }

        self.update_global_data_representations();
    }

    /// Sets the translation and rotation, and updates the velocity of the rigid body motion with
    /// finite differences, using the previous translation and rotation
    pub fn set_translation_and_rotation_with_finite_difference_for_the_velocity(
        &mut self,
        time_step: Float,
        translation: SpatialVector,
        rotation: SpatialVector
    ) {
        self.rigid_body_motion.update_translation_with_velocity_using_finite_difference(
            translation, 
            time_step
        );

        self.rigid_body_motion.update_rotation_with_velocity_using_finite_difference(
            rotation, 
            time_step
        );

        self.update_global_data_representations();
    }

    /// Sets the translation, without changing the rotation or the velocity
    pub fn set_translation_only(
        &mut self,
        translation: SpatialVector
    ) {
        self.rigid_body_motion.translation = translation;

        self.update_global_data_representations();
    }

    /// Sets the rotation, without changing the translation or the velocity
    pub fn set_rotation_only(
        &mut self,
        rotation: SpatialVector
    ) {
        self.rigid_body_motion.rotation = rotation;

        self.update_global_data_representations();
    }

    /// Sets the translation and rotation, without changing the velocity
    pub fn set_translation_and_rotation(
        &mut self,
        translation: SpatialVector,
        rotation: SpatialVector
    ) {
        self.rigid_body_motion.translation = translation;
        self.rigid_body_motion.rotation = rotation;

        self.update_global_data_representations();
    }

    /// Returns true if any of the wings have a dynamic stall model
    pub fn has_dynamic_stall(&self) -> bool {
        self.dynamic_stall.iter().any(|model| model.is_some())
    }

    /// Sets the time step to use in the dynamic stall models for the next evaluations. Should be
    /// called before solving for the circulation strength in a new time step.
    pub fn set_dynamic_stall_time_step(&mut self, time_step: Float) {
        for model in self.dynamic_stall.iter_mut().flatten() {
            model.set_time_step(time_step);
        }
    }

    /// Resets the state of the dynamic stall models, so that they give static values until the
    /// next completed time step.
    pub fn reset_dynamic_stall(&mut self) {
        for model in self.dynamic_stall.iter_mut().flatten() {
            model.reset();
        }
    }

    /// Updates the state of the dynamic stall models after a completed time step. Should be called
    /// once per time step, with the force input from the final solution of the time step, and
    /// before the time step of the dynamic stall models is changed.
    ///
    /// The angles of attack and the internal states are taken directly from the force input, so
    /// that the state is based on the same values as the forces, even if the geometry or the
    /// internal states of the model have been changed afterwards, e.g., by a controller. Only the
    /// magnitude of the velocity is used, so the coordinate system of the force input does not
    /// matter.
    pub fn update_dynamic_stall_state(&mut self, force_input: &SectionalForcesInput) {
        if !self.has_dynamic_stall() {
            return;
        }

        for wing_index in 0..self.nr_wings() {
            let Some(segment_states) = self.wing_dynamic_stall_states(
                wing_index,
                &force_input.angles_of_attack,
                &force_input.section_models_internal_state,
                &force_input.velocity
            ) else {
                continue;
            };

            if let Some(model) = &mut self.dynamic_stall[wing_index] {
                model.commit(segment_states);
            }
        }
    }
}
