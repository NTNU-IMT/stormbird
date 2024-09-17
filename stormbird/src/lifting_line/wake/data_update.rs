use super::*;


/// This code block contains the logic to update the wake structure, which primarily happens after a
/// time step has been completed.
impl Wake {
    /// Takes a line force vector as input, that might have a different position and orientation 
    /// than the current model, and updates the relevant internal geometry
    ///
    /// # Argument
    /// * `line_force_model` - The line force model that the wake is based on
    pub fn synchronize_wing_geometry_before_time_step(&mut self, line_force_model: &LineForceModel) {
        let span_points = line_force_model.span_points();

        for i in 0..span_points.len() {
            self.points[i] = span_points[i];
        }
    }

    /// Update the wake geometry and strength based on the final solution at a time step.
    /// 
    /// This will:
    /// 1) stream the wake points downstream
    /// 2) stream the strength downstream
    pub fn update_after_completed_time_step(
        &mut self, 
        new_circulation_strength: &[f64], 
        time_step: f64, 
        line_force_model: &LineForceModel,
        ctrl_points_freestream: &[SpatialVector<3>],
        wake_points_freestream: &[SpatialVector<3>]
    ) {
        self.update_line_force_model_data(line_force_model, ctrl_points_freestream);

        self.update_wake_points_after_completed_time_step(
            time_step, 
            line_force_model,
            wake_points_freestream
        );

        self.update_strength_after_completed_time_step(new_circulation_strength);

        self.update_panel_lifetime(time_step);
        self.update_panel_strength_damping_factor();

        self.number_of_time_steps_completed += 1;
    }

    

    /// Update the strength of the wake panels closest to the wing geometry.
    /// 
    /// This is the same as updating the circulation strength on the first panels in the wake.
    pub fn update_wing_strength(&mut self, new_circulation_strength: &[f64]) {
        for i in 0..new_circulation_strength.len() {
            self.undamped_strengths[i] = new_circulation_strength[i];
        }
    }

    /// Update the lifetime of the panels in the wake.
    fn update_panel_lifetime(&mut self, time_step: f64) {
        for i_stream in (1..self.indices.nr_panels_per_line_element).rev() {
            for i_span in 0..self.indices.nr_panels_along_span {
                let current_index  = self.indices.panel_index(i_stream, i_span);
                let previous_index = self.indices.panel_index(i_stream - 1, i_span);

                self.panels_lifetime[current_index] = self.panels_lifetime[previous_index] + time_step;
            }
        }
    }

    fn update_panel_strength_damping_first_panels(&mut self) {
        for i_span in 0..self.indices.nr_panels_along_span {
            let current_index = self.indices.panel_index(0, i_span);

            let amount_of_flow_separation = self.line_force_model_data.amount_of_flow_separation[i_span];

            let damping_strength = if let Some(strength_damping_factor_separated) = self.settings.strength_damping_factor_separated {
                self.settings.strength_damping_factor * (1.0 - amount_of_flow_separation) +
                strength_damping_factor_separated * amount_of_flow_separation
            } else {
                self.settings.strength_damping_factor
            };

            self.panels_strength_damping_factor[current_index] = damping_strength;
        }
    }

    fn update_panel_strength_damping_factor(&mut self) {
        for i_stream in (1..self.indices.nr_panels_per_line_element).rev() {
            for i_span in 0..self.indices.nr_panels_along_span {
                let current_index  = self.indices.panel_index(i_stream, i_span);
                let previous_index = self.indices.panel_index(i_stream - 1, i_span);

                self.panels_strength_damping_factor[current_index] = self.panels_strength_damping_factor[previous_index];
            }
        }

        self.update_panel_strength_damping_first_panels();
    }

    /// Moves the first wake points after the wing geometry itself.
    /// 
    /// How the points are moved depends on both the sectional force model for each wing and - in 
    /// some cases - the angle of attack on each line force model.
    fn move_first_free_wake_points(
        &mut self, 
        line_force_model: &LineForceModel,
    ) {                
        // Extract relevant information from the line force model
        let span_lines = line_force_model.span_lines();
        let wake_angles     = line_force_model.wake_angles(&self.line_force_model_data.ctrl_points_velocity);

        // Compute a change vector based on ctrl point data
        let mut ctrl_points_change_vector: Vec<SpatialVector<3>> = Vec::with_capacity(
            self.indices.nr_panels_along_span
        );

        for i in 0..self.indices.nr_panels_along_span {
            let amount_of_flow_separation = self.line_force_model_data.amount_of_flow_separation[i];
            
            // Little flow separation means that the ctrl point should move in the direction of the
            // chord vector. Large flow separation means that the ctrl point should move in the
            // direction of the velocity vector, but with an optional rotation around the axis of
            // the span line.
            let velocity_direction = self.line_force_model_data.ctrl_points_velocity[i].rotate_around_axis(
                wake_angles[i], 
                span_lines[i].relative_vector().normalize()
            ).normalize();

            let wake_direction = if self.settings.use_chord_direction {
                let chord_direction = self.line_force_model_data.chord_vectors[i].normalize();

                (
                    velocity_direction * amount_of_flow_separation + 
                    chord_direction * (1.0 - amount_of_flow_separation)
                ).normalize()
            } else {
                velocity_direction
            };

            ctrl_points_change_vector.push(
                self.settings.first_panel_relative_length * self.line_force_model_data.chord_vectors[i].length() * wake_direction
            );
        }

        // Transfer ctrl point data to span lines
        let span_points_change_vector = line_force_model.span_point_values_from_ctrl_point_values(
            &ctrl_points_change_vector, true
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

    /// Update the wake points by streaming them downstream.
    /// 
    /// The first and second "rows" - meaning the wing geometries and the first row of wake points -
    /// are treaded as special cases. The rest are moved based on the euler method
    fn update_wake_points_after_completed_time_step(
        &mut self, 
        time_step: f64,
        line_force_model: &LineForceModel,
        wake_points_freestream: &[SpatialVector<3>]
    ) {
        self.move_first_free_wake_points(line_force_model);
        self.stream_free_wake_points(time_step, wake_points_freestream);
        self.move_last_wake_points(line_force_model, wake_points_freestream);
    }


    /// Moves the last points in the wake based on the chord length and the freestream velocity
    /// 
    /// # Arguments
    /// * `line_force_model` - The line force model that the wake is based on
    /// * `wake_points_freestream` - The freestream velocity at the wake points
    pub fn move_last_wake_points(
        &mut self,
        line_force_model: &LineForceModel,
        wake_points_freestream: &[SpatialVector<3>]
    ) {
        let start_index_last = self.points.len() - self.indices.nr_points_along_span;
        let start_index_previous = start_index_last - self.indices.nr_points_along_span;

        let chord_vectors = line_force_model.span_point_values_from_ctrl_point_values(
            &self.line_force_model_data.chord_vectors, true
        );

        for i in 0..self.indices.nr_points_along_span {
            let current_velocity = wake_points_freestream[start_index_last + i];
            let change_vector = self.settings.last_panel_relative_length * chord_vectors[i].length() * current_velocity.normalize();

            self.points[start_index_last + i] = self.points[start_index_previous + i] + change_vector;
        }
    }

    /// Stream all free wake points based on the Euler method.
    fn stream_free_wake_points(&mut self, time_step: f64, wake_points_freestream: &[SpatialVector<3>]) {
        let velocity = self.velocity_at_wake_points(wake_points_freestream);

        for i_stream in (2..self.indices.nr_points_per_line_element).rev() {
            for i_span in 0..self.indices.nr_points_along_span {
                let previous_flat_index = self.indices.point_index(i_stream - 1, i_span);
                let current_flat_index  = self.indices.point_index(i_stream, i_span);
                
                let previous_wake_point = self.points[previous_flat_index];
                let previous_velocity = velocity[previous_flat_index];

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

    /// Shift strength values downstream and update the wing values with the new circulation
    /// 
    /// Principle: the strength of each panel is updated to be the same as the previous panel in the
    /// stream wise direction in the last time step.
    ///
    /// # Argument
    /// * `new_circulation_strength` - The new circulation strength for the wing
    fn update_strength_after_completed_time_step(&mut self, new_circulation_strength: &[f64]) {
        // Stream old strength values downstream
        for i_stream in (1..self.indices.nr_panels_per_line_element).rev() {
            for i_span in 0..self.indices.nr_panels_along_span {
                let current_index  = self.indices.panel_index(i_stream, i_span);
                let previous_index = self.indices.panel_index(i_stream - 1, i_span);

                self.undamped_strengths[current_index] = self.undamped_strengths[previous_index];
            }
        }

        // Update the strength of the first panels
        self.update_wing_strength(new_circulation_strength);

        // Apply damping to the strength
        self.appply_damping_to_strength();
    }

    fn appply_damping_to_strength(&mut self) {
        for i in 0..self.indices.nr_panels() {
            let damping_factor = (-self.panels_lifetime[i] * self.panels_strength_damping_factor[i]).exp();

            self.strengths[i] = self.undamped_strengths[i] * damping_factor;
        }
    }
}