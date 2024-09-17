use super::*;

/// This code block contains the logic for calculating the induced velocities from the wake panels.
impl Wake {
    /// Calculates the induced velocities from all the panels in the wake
    ///
    /// # Arguments
    /// * `points` - The points at which the induced velocities are calculated
    /// the off-body viscous core length in the potential theory model, if it exists.
    pub fn induced_velocities(
        &self, 
        points: &[SpatialVector<3>], 
    ) -> Vec<SpatialVector<3>> {
        self.induced_velocities_local(
            points, 0, 
            self.strengths.len(), 
            false)
    }

    /// Calculates the induced velocity from the first panels in the stream wise direction only. This
    /// is used to calculate the velocity at the control points in the strength solver more 
    /// efficiently, as each iteration only updates the strength of these panels.
    ///
    /// # Arguments
    /// * `points` - The points at which the induced velocities are calculated
    /// the off-body viscous core length in the potential theory model if it exists.
    pub fn induced_velocities_from_first_panels(
        &self, points: &[SpatialVector<3>], 
    ) -> Vec<SpatialVector<3>> {
        self.induced_velocities_local(
            points, 
            0, 
            self.indices.nr_panels_along_span, 
            self.settings.neglect_self_induced_velocities
        )
    }

    /// Calculates the induced velocities from all the panels in the free wake, neglecting the first 
    /// panels, at the input points. 
    ///
    /// # Arguments
    /// * `points` - The points at which the induced velocities are calculated
    /// the off-body viscous core length in the potential theory model if it exists.
    pub fn induced_velocities_from_free_wake(
        &self, 
        points: &[SpatialVector<3>], 
    ) -> Vec<SpatialVector<3>> {
        self.induced_velocities_local(
            points, 
            self.indices.nr_panels_along_span, 
            self.strengths.len(),
            self.settings.neglect_self_induced_velocities
        )
    }

    /// Calculates induced velocities from the panels starting at start_index and ending at end_index
    fn induced_velocities_local(
        &self, 
        points: &[SpatialVector<3>], 
        start_index: usize, 
        end_index: usize, 
        neglect_self_induced: bool
    ) -> Vec<SpatialVector<3>> {
        points.par_iter()
            .enumerate()
            .map(|(point_index, point)| {
                (start_index..end_index).into_iter().map(|i_panel| {
                    if neglect_self_induced {
                        let (_stream_index, span_index) = self.indices.reverse_panel_index(i_panel);

                        let wing_index_panel = self.wing_index(span_index);
                        let wing_index_point = self.wing_index(point_index);

                        if wing_index_panel == wing_index_point {
                            SpatialVector::<3>::default()
                        } else {
                            self.induced_velocity_from_panel(i_panel, *point)
                        }

                    } else {
                        self.induced_velocity_from_panel(i_panel, *point)
                    }
                }).sum()
            }).collect()
    }

    #[inline(always)]
    /// Calculates the induced velocity from a single panel at the input point with unit strength
    pub fn unit_strength_induced_velocity_from_panel(
        &self, 
        stream_index: usize,
        span_index: usize,
        point: SpatialVector<3>, 
    ) -> SpatialVector<3> {
        let panel_points = self.panel_points(stream_index, span_index);

        let flat_index = self.indices.panel_index(stream_index, span_index);

        let viscous_core_length = self.panels_viscous_core_length[flat_index];

        self.potential_theory_model.induced_velocity_from_panel_with_unit_strength(
            &panel_points, 
            point,
            viscous_core_length
        )
    }

    #[inline(always)]
    /// Calculates the induced velocity from a single panel at the input point with unit strength
    pub fn unit_strength_induced_velocity_from_panel_flat_index(
        &self, 
        panel_index: usize, 
        point: SpatialVector<3>, 
    ) -> SpatialVector<3> {
        let (stream_index, span_index) = self.indices.reverse_panel_index(panel_index);

        self.unit_strength_induced_velocity_from_panel(stream_index, span_index, point)
    }

    #[inline(always)]
    /// Calculates the induced velocity from a single panel at the input point
    fn induced_velocity_from_panel(&self, panel_index: usize, point: SpatialVector<3>) -> SpatialVector<3> {
        if self.strengths[panel_index] == 0.0 {
            SpatialVector::<3>::default()
        } else {
            let unit_velocity = self.unit_strength_induced_velocity_from_panel_flat_index(panel_index, point);

            self.strengths[panel_index] * unit_velocity
        }
    }

    /// Returns the velocity at all the wake points.
    ///
    /// The velocity is calculated as the sum of the freestream velocity and the induced velocity.
    /// However, if the settings contains and end-index for the induced velocities, the induced
    /// velocities can be neglected for the last panels. This is useful for speeding up simulations.
    ///
    /// # Argument
    /// * `wake_points_freestream` - A vector containing the freestream velocity at the wake points
    pub fn velocity_at_wake_points(&self, wake_points_freestream: &[SpatialVector<3>]) -> Vec<SpatialVector<3>> {
        let mut velocity: Vec<SpatialVector<3>> = wake_points_freestream.to_vec();

        let end_index = self.settings.end_index_induced_velocities_on_wake.min(self.points.len());

        if end_index > 0 && self.number_of_time_steps_completed > 2 {
            let u_i_calc: Vec<SpatialVector<3>> = self.induced_velocities(&self.points[0..end_index]);

            for i in 0..end_index {
                velocity[i] += u_i_calc[i];
            }
        }

        velocity
    }

}