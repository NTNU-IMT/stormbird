// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use super::*;

use crate::line_force_model::LineForceModel;

impl DynamicWake {
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

        let wake_points_freestream = vec![
            wake_building_velocity; self.points.len()
        ];

        self.synchronize_first_points_to_wing_geometry(line_force_model);

        let nr_initial_time_steps = self.indices.nr_points_per_line_element;

        for _ in 0..nr_initial_time_steps {
            self.update_wake_points_before_solving(
                time_step,
                &line_force_model,
                &wake_points_freestream
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

        let chord_vectors = &line_force_model.chord_vectors_global_at_span_points;

        let average_chord_vector = chord_vectors.iter()
            .sum::<SpatialVector>() / chord_vectors.len() as Float;

        let average_chord_length = average_chord_vector.length();

        let wake_length = relative_length_factor * average_chord_length;

        let time_step = 1.0;
        let wake_building_velocity = (wake_length / nr_panels_per_line_element as Float) * average_chord_vector;

        self.initialize_with_velocity_and_time_step(
            line_force_model, 
            wake_building_velocity, 
            time_step
        );

    }
}