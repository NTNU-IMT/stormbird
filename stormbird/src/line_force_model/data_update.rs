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

    pub fn update_calculated_values_after_create(&mut self) {
        self.update_global_data_representations();
        self.update_ctrl_point_spanwise_distance();
    }

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

    pub fn update_ctrl_points(&mut self) {
        if self.ctrl_points_global.len() != self.span_lines_global.len() {
            self.ctrl_points_global = vec![SpatialVector::from([0.0, 0.0, 0.0]); self.span_lines_global.len()];
        }

        for i in 0..self.span_lines_global.len() {
            self.ctrl_points_global[i] = self.span_lines_global[i].ctrl_point();
        }
    }

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

            for i in 0..self.wing_indices[wing_index].end - self.wing_indices[wing_index].start {
                let relative_distance = current_wing_span_distance[i] / total_distance;

                self.ctrl_point_spanwise_distance.push(current_wing_span_distance[i] - 0.5 * total_distance);
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

    pub fn set_section_models_internal_state(&mut self, internal_state: &[Float]) {
        for wing_index in 0..self.nr_wings() {
            match self.section_models[wing_index] {
                SectionModel::Foil(_) => {}
                SectionModel::VaryingFoil(ref mut foil) => {
                    foil.current_internal_state = internal_state[wing_index];
                }
                SectionModel::RotatingCylinder(ref mut cylinder) => {
                    cylinder.revolutions_per_second = internal_state[wing_index];
                },
                SectionModel::EffectiveWindSensor => {}
            }
        }
    }

    pub fn set_controller_output(&mut self, controller_output: &[ControllerOutput]) {
        let local_wing_angles: Vec<Float> = controller_output.iter()
            .map(|v| v.local_wing_angle).collect();
        
        let section_models_internal_state: Vec<Float> = controller_output.iter()
            .map(|v| v.section_model_internal_state).collect();
        
        self.set_local_wing_angles(&local_wing_angles);
        self.set_section_models_internal_state(&section_models_internal_state);
    }

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

    pub fn set_translation_only(
        &mut self,
        translation: SpatialVector
    ) {
        self.rigid_body_motion.translation = translation;

        self.update_global_data_representations();
    }

    pub fn set_rotation_only(
        &mut self,
        rotation: SpatialVector
    ) {
        self.rigid_body_motion.rotation = rotation;

        self.update_global_data_representations();
    }

    pub fn set_translation_and_rotation(
        &mut self,
        translation: SpatialVector,
        rotation: SpatialVector
    ) {
        self.rigid_body_motion.translation = translation;
        self.rigid_body_motion.rotation = rotation;

        self.update_global_data_representations();
    }
}
