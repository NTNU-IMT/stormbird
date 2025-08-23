 use super::*;

 impl Wake {
    /// Function to initialize shape and strength of the wake, where the length is determined based
    /// on the input velocity and time step. This can be used at the beginning of a simulation.
    pub fn initialize_with_velocity_and_time_step(
        &mut self, 
        line_force_model: &LineForceModel, 
        wake_building_velocity: SpatialVector, 
        time_step: Float
    ) {
        let nr_panels = self.indices.nr_panels();
        
        self.strengths = vec![0.0; nr_panels];

        self.velocity_at_points = vec![wake_building_velocity; self.points.len()];

        let ctrl_points_freestream = vec![wake_building_velocity; line_force_model.nr_span_lines()];

        let line_force_model_data = LineForceModelData::new(
            line_force_model,
            &ctrl_points_freestream,
            &ctrl_points_freestream
        );

        self.synchronize_first_points_to_wing_geometry(line_force_model);

        let nr_initial_time_steps = self.indices.nr_points_per_line_element;

        for _ in 0..nr_initial_time_steps {
            self.update_wake_points(
                time_step,
                &line_force_model,
                &line_force_model_data
            );
        }

        self.update_panel_data();
    }


    /// Simple initialization based only on the chord length of the line force model. This avoids 
    /// having to specify a velocity and time step to create the wake.
    pub fn initialize_based_on_chord_length(
        &mut self, 
        line_force_model: &LineForceModel,
        relative_length_factor: Float,
    ) {
        let nr_panels_per_line_element = self.indices.nr_panels_per_line_element;

        let chord_vectors = line_force_model.global_chord_vectors();

        let average_chord_vector = chord_vectors.iter()
            .sum::<SpatialVector>() / chord_vectors.len() as Float;

        let average_chord_length = average_chord_vector.length();

        let wake_length = relative_length_factor * average_chord_length;

        let time_step = 1.0;
        let wake_building_velocity = (wake_length / nr_panels_per_line_element as Float) * average_chord_vector;

        self.initialize_with_velocity_and_time_step(line_force_model, wake_building_velocity, time_step);

    }
}