use super::*;

/// This code block contains the logic to update the wake structure
impl Wake {
    /// Function that updates the wake data before the solver executes a new time step. The main job
    /// is to ensure that geometry of the wake and the strength behind the first panels is correct.
    /// 
    /// # Steps that are performed in this function
    /// Since the last time step, the line force model might have moved, and the wake points have 
    /// streamed downstream. That means that the following must happen:
    /// - Synchronize the geometry for the first points in the wake to the geometry of the line 
    /// force model.
    /// - Stream the old wake points downstream.
    /// - Shift the strength of the panels downstream.
    /// 
    /// # Arguments
    /// * `time_step` - The current value of the time step
    /// * `line_force_model` - The line force model that the wake is based on
    /// * `wake_points_freestream` - The freestream velocity at the points in the wake for the 
    /// current time step
    pub fn update_before_solving(
        &mut self,
        time_step: f64,
        line_force_model: &LineForceModel,
        line_force_model_data: &LineForceModelData,
    ) {
        self.update_wake_points(time_step, line_force_model, line_force_model_data);
        self.update_panel_data();

        self.stream_strength_values_downstream();
    }

    /// Update the wake geometry and strength based on the final solution at a time step.
    ///
    /// This will:
    /// 1) stream the wake points downstream
    /// 2) stream the strength downstream
    pub fn update_after_solving(
        &mut self,
        new_circulation_strength: &[f64],
        wake_points_freestream: &[SpatialVector<3>],
    ) {
        self.update_wing_strength(new_circulation_strength);

        self.update_velocity_at_points(wake_points_freestream);

        self.number_of_time_steps_completed += 1;
    }

    /// Update the wake points by streaming them downstream.
    ///
    /// The first and second "rows" - meaning the wing geometries and the first row of wake points -
    /// are treaded as special cases. The rest are moved based on the euler method
    pub fn update_wake_points(
        &mut self,
        time_step: f64,
        line_force_model: &LineForceModel,
        line_force_model_data: &LineForceModelData,
    ) {
        self.synchronize_first_points_to_wing_geometry(line_force_model);
        self.stream_free_wake_points_based_on_stored_velocity(time_step);
        self.move_first_free_wake_points(line_force_model_data);
        self.move_last_wake_points(line_force_model_data);
    }

    /// Takes a line force vector as input, that might have a different position and orientation
    /// than the previous time step, and updates the first points in the wake to match the new 
    /// geometry.
    ///
    /// # Argument
    /// * `line_force_model` - The line force model that the wake is based on
    pub fn synchronize_first_points_to_wing_geometry(&mut self, line_force_model: &LineForceModel) {
        let span_points = line_force_model.span_points();

        for i in 0..span_points.len() {
            self.points[i] = span_points[i];
        }
    }

    /// Recalculates the panel data based on the current geometry of the wake.
    pub fn update_panel_data(&mut self) {
        for i in 0..self.indices.nr_panels() {
            let (stream_index, span_index) = self.indices.reverse_panel_index(i);

            let panel_points = self.panel_points(stream_index, span_index);
            
            self.panels[i] = Panel::new(
                panel_points,
                self.potential_theory_settings.far_field_ratio,
                self.panels_viscous_core_length[i]
            );
            
        }
    }

    /// Calculates the velocity at the wake points based on the current state of the wake.
    pub fn update_velocity_at_points(&mut self, wake_points_freestream: &[SpatialVector<3>]) {
        self.velocity_at_points = self.velocity_at_wake_points(wake_points_freestream);
    }

    /// Update the strength of the wake panels closest to the wing geometry.
    ///
    /// This is the same as updating the circulation strength on the first panels in the wake.
    pub fn update_wing_strength(&mut self, new_circulation_strength: &[f64]) {
        for i in 0..new_circulation_strength.len() {
            self.strengths[i] = new_circulation_strength[i];
        }
    }

    /// Moves the first wake points after the wing geometry itself.
    ///
    /// How the points are moved depends on both the sectional force model for each wing and - in
    /// some cases - the angle of attack on each line force model.
    /// 
    /// In general, the principle is that the first free wake points are moved from the wing 
    /// geometry and then *either* in the direction of the chord vector or the velocity vector. 
    /// Which vector to use as a direction depends on the `amount_of_flow_separation` value and 
    /// wether the `use_chord_direction` setting is set to true or false.
    fn move_first_free_wake_points(
        &mut self,
        line_force_model_data: &LineForceModelData,
    ) {
        // Compute a change vector based on ctrl point data
        let mut ctrl_points_change_vector: Vec<SpatialVector<3>> = Vec::with_capacity(
            self.indices.nr_points_along_span
        );

        for i in 0..self.indices.nr_panels_along_span {

            // Small flow separation means that the ctrl point should move in the direction of the
            // chord vector. Large flow separation means that the ctrl point should move in the
            // direction of the velocity vector, but with an optional rotation around the axis of
            // the span line.
            let velocity_direction = if line_force_model_data.wake_angles[i] == 0.0 {
                line_force_model_data.felt_ctrl_points_freestream[i].normalize()
            } else {
                line_force_model_data.felt_ctrl_points_freestream[i]
                    .rotate_around_axis(
                        line_force_model_data.wake_angles[i],
                        line_force_model_data.span_lines[i].relative_vector().normalize()
                    ).normalize()
            };

            let wake_direction = if self.settings.use_chord_direction {
                let amount_of_flow_separation = line_force_model_data.amount_of_flow_separation[i];

                let chord_direction = line_force_model_data.chord_vectors[i].normalize();

                velocity_direction * amount_of_flow_separation +
                chord_direction * (1.0 - amount_of_flow_separation)
            } else {
                velocity_direction
            };

            ctrl_points_change_vector.push(
                self.settings.first_panel_relative_length * line_force_model_data.chord_vectors[i].length() * wake_direction
            );
        }

        // Transfer ctrl point data to span point data
        let span_points_change_vector = line_force_model_data.span_point_values_from_ctrl_point_values(
            &ctrl_points_change_vector, false
        );

        // Update the wake points
        let old_start_index = self.indices.nr_points_along_span;
        let old_end_index   = 2 * self.indices.nr_points_along_span;

        let old_wake_points = self.points[old_start_index..old_end_index].to_vec();

        for i in 0..self.indices.nr_points_along_span {
            let estimated_new_wake_point = self.points[i] + span_points_change_vector[i];

            self.points[i + self.indices.nr_points_along_span] =
                old_wake_points[i] * self.settings.shape_damping_factor +
                estimated_new_wake_point * (1.0 - self.settings.shape_damping_factor);
        }
    }

    /// Moves the last points in the wake based.
    ///
    /// The length of the last panel is determined based on the `last_panel_relative_length`
    /// parameter in the settings, plus the chord length. The direction of the last panel is taken
    /// to be the same as the direction between the previous two points in the wake.
    ///
    /// # Arguments
    /// * `line_force_model_data` - The line force model data that the should use for the update
    pub fn move_last_wake_points(
        &mut self,
        line_force_model_data: &LineForceModelData,
    ) { 
        let nr_points = self.points.len();
        let nr_points_along_span = self.indices.nr_points_along_span;

        let multi_panel_wake = self.indices.nr_panels_per_line_element > 1;

        let start_index_last = nr_points - nr_points_along_span;
        let start_index_previous = start_index_last - nr_points_along_span;
        
        let chord_vectors = line_force_model_data.span_point_values_from_ctrl_point_values(
            &line_force_model_data.chord_vectors, true
        );

        let felt_ctrl_points_freestream = line_force_model_data.span_point_values_from_ctrl_point_values(
            &line_force_model_data.felt_ctrl_points_freestream, true
        );

        for i in 0..self.indices.nr_points_along_span {
            let change_direction = if multi_panel_wake {
                let previous_point        = self.points[start_index_previous + i];
                let second_previous_point = self.points[start_index_previous - nr_points_along_span + i];

                (previous_point - second_previous_point).normalize()
            } else {
                felt_ctrl_points_freestream[i].normalize()
            };

            let change_vector = self.settings.last_panel_relative_length * chord_vectors[i].length() * change_direction;

            self.points[start_index_last + i] = self.points[start_index_previous + i] + change_vector;
        }
    }

    /// Stream all free wake points based on the Euler method.
    fn stream_free_wake_points_based_on_stored_velocity(&mut self, time_step: f64) {
        for i_stream in (2..self.indices.nr_points_per_line_element).rev() {
            for i_span in 0..self.indices.nr_points_along_span {
                let previous_flat_index = self.indices.point_index(i_stream - 1, i_span);
                let current_flat_index  = self.indices.point_index(i_stream, i_span);

                let previous_wake_point = self.points[previous_flat_index];
                let previous_velocity = self.velocity_at_points[previous_flat_index];

                let integrated_point = previous_wake_point + time_step * previous_velocity;

                if self.settings.shape_damping_factor > 0.0 {
                    let current_wake_point = self.points[current_flat_index];

                    self.points[current_flat_index] = current_wake_point * self.settings.shape_damping_factor +
                        integrated_point * (1.0 - self.settings.shape_damping_factor);
                } else {
                    self.points[current_flat_index] = integrated_point;
                }
            }
        }
    }

    /// Shifts the strength of the panels downstream. 
    pub fn stream_strength_values_downstream(&mut self) {
        for i_stream in (1..self.indices.nr_panels_per_line_element).rev() {
            for i_span in 0..self.indices.nr_panels_along_span {
                let current_index  = self.indices.panel_index(i_stream, i_span);
                let previous_index = self.indices.panel_index(i_stream - 1, i_span);

                self.strengths[current_index] = self.strengths[previous_index];
            }
        }
    }
}
