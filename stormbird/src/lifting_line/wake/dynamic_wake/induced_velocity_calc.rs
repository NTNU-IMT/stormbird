// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)


use super::*;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// This code block contains the logic for calculating the induced velocities from the wake panels.
impl DynamicWake {
    /// Calculates the induced velocities from all the panels in the wake
    ///
    /// # Arguments
    /// * `points` - The points at which the induced velocities are calculated
    /// the off-body viscous core length in the potential theory model, if it exists.
    pub fn induced_velocities(
        &self, 
        points: &[SpatialVector], 
    ) -> Vec<SpatialVector> {
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
        &self, points: &[SpatialVector], 
    ) -> Vec<SpatialVector> {
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
        points: &[SpatialVector], 
    ) -> Vec<SpatialVector> {
        self.induced_velocities_local(
            points, 
            self.indices.nr_panels_along_span, 
            self.strengths.len(),
            self.settings.neglect_self_induced_velocities
        )
    }

    #[cfg(not(feature = "parallel"))]
    /// Calculate the induced velocities from panels on all points, including self-induced 
    /// velocities
    fn induced_velocities_local_include_self_induced(
        &self, 
        points: &[SpatialVector], 
        start_index: usize, 
        end_index: usize
    ) -> Vec<SpatialVector> {
        let mut results = vec![SpatialVector::default(); points.len()];

        for panel_index in start_index..end_index {
            let strength = self.strengths[panel_index];
            
            if strength != 0.0 {
                for (point_index, &point) in points.iter().enumerate() {
                    results[point_index] += strength * self.unit_strength_induced_velocity_from_panel_flat_index(panel_index, point);
                }
            }
        }

        results
    }

    #[cfg(not(feature = "parallel"))]
    /// Calculate the induced velocities from panels on all points, including self-induced 
    /// velocities
    fn _induced_velocities_local_include_self_induced_old(
        &self, 
        points: &[SpatialVector], 
        start_index: usize, 
        end_index: usize
    ) -> Vec<SpatialVector> {
        // Parallelize over points instead of panels to avoid race conditions
        (0..points.len()).into_iter()
            .map(|point_index| {
                let point = points[point_index];
                (start_index..end_index)
                    .map(|panel_index| self.induced_velocity_from_panel(panel_index, point))
                    .fold(SpatialVector::default(), |acc, velocity| acc + velocity)
            })
            .collect()
    }

    #[cfg(feature = "parallel")]
    /// Calculate the induced velocities from panels on all points, including self-induced 
    /// velocities
    fn induced_velocities_local_include_self_induced(
        &self, 
        points: &[SpatialVector], 
        start_index: usize, 
        end_index: usize
    ) -> Vec<SpatialVector> {
        // Parallelize over points instead of panels to avoid race conditions
        (0..points.len()).into_par_iter()
            .map(|point_index| {
                let point = points[point_index];
                (start_index..end_index)
                    .map(|panel_index| self.induced_velocity_from_panel(panel_index, point))
                    .fold(SpatialVector::default(), |acc, velocity| acc + velocity)
            })
            .collect()
    }

    fn induced_velocity_local_neglect_self_induced(
        &self, 
        points: &[SpatialVector], 
        start_index: usize, 
        end_index: usize, 
    ) -> Vec<SpatialVector> {
        let mut results = vec![SpatialVector::default(); points.len()];

        for i_panel in start_index..end_index {
            let (_stream_index, span_index) = self.indices.reverse_panel_index(i_panel);
            let wing_index_panel = self.wing_index(span_index);

            for (point_index, &point) in points.iter().enumerate() {
                let wing_index_point = self.wing_index(point_index);

                if wing_index_panel != wing_index_point {
                    results[point_index] += self.induced_velocity_from_panel(i_panel, point);
                }
            }
        }

        results
    }

    /// Calculates induced velocities from the panels starting at start_index and ending at end_index
    fn induced_velocities_local(
        &self, 
        points: &[SpatialVector], 
        start_index: usize, 
        end_index: usize, 
        neglect_self_induced: bool
    ) -> Vec<SpatialVector> {
        if neglect_self_induced {
            self.induced_velocity_local_neglect_self_induced(points, start_index, end_index)
        } else {
            self.induced_velocities_local_include_self_induced(points, start_index, end_index)
        }
    }

    #[inline(always)]
    /// Calculates the induced velocity from a single panel at the input point with unit strength
    pub fn unit_strength_induced_velocity_from_panel(
        &self, 
        stream_index: usize,
        span_index: usize,
        point: SpatialVector, 
    ) -> SpatialVector {
        let flat_index = self.indices.panel_index(stream_index, span_index);

        self.unit_strength_induced_velocity_from_panel_flat_index(flat_index, point)
    }

    #[inline(always)]
    /// Calculates the induced velocity from a single panel at the input point with unit strength
    pub fn unit_strength_induced_velocity_from_panel_flat_index(
        &self, 
        panel_index: usize, 
        point: SpatialVector, 
    ) -> SpatialVector {
        let u_i = self.panels[panel_index].induced_velocity_with_unit_strength(point);

        let point_mirrored = self.potential_theory_settings.symmetry_condition.mirrored_point(point);

        if let Some(point_mirrored) = point_mirrored {
            let u_i_m = self.panels[panel_index].induced_velocity_with_unit_strength(point_mirrored);

            self.potential_theory_settings.symmetry_condition.corrected_velocity(u_i, u_i_m)
        } else {
            u_i
        }
    }

    #[inline(always)]
    /// Calculates the induced velocity from a single panel at the input point
    fn induced_velocity_from_panel(&self, panel_index: usize, point: SpatialVector) -> SpatialVector {
        if self.strengths[panel_index] == 0.0 {
            SpatialVector::default()
        } else {
            let unit_velocity = self.unit_strength_induced_velocity_from_panel_flat_index(panel_index, point);

            self.strengths[panel_index] * unit_velocity
        }
    }
}