// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)


use super::*;

use stormath::matrix::Matrix;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// This code block contains the logic for calculating the induced velocities from the wake panels.
///
/// The wake does not know which wing a point belongs to. Splitting the induced velocities into
/// self-induced and other-induced velocities is done in the
/// [`FrozenWake`](crate::lifting_line::wake::frozen_wake::FrozenWake), based on the velocities from
/// the panels shed from each line element, see
/// [`induced_velocities_from_free_wake_per_line_element`](Self::induced_velocities_from_free_wake_per_line_element).
impl DynamicWake {
    /// Calculates the total induced velocities from all the panels in the wake
    ///
    /// # Arguments
    /// * `points` - The points at which the induced velocities are calculated
    pub fn induced_velocities(
        &self,
        points: &[SpatialVector],
    ) -> Vec<SpatialVector> {
        self.induced_velocities_from_panel_range(
            points,
            0,
            self.strengths.len(),
        )
    }

    /// Calculates the velocities induced by the free wake, i.e., all panels except the first row
    /// right behind the wings, separately for the panels shed from each line element.
    ///
    /// The result is written to `out`, where each row corresponds to a point and each column to a
    /// line element. That is, `out[[point_index, span_index]]` is the sum of the velocities induced
    /// at the point by all the free panels with the given span index. The total velocity at a point
    /// is the sum of its row.
    ///
    /// # Arguments
    /// * `points` - The points at which the induced velocities are calculated
    /// * `out` - Matrix with shape (points.len(), nr_panels_along_span), which is overwritten
    pub fn induced_velocities_from_free_wake_per_line_element(
        &self,
        points: &[SpatialVector],
        out: &mut Matrix<SpatialVector>,
    ) {
        let nr_cols = self.indices.nr_panels_along_span;

        assert_eq!(
            out.shape, [points.len(), nr_cols],
            "The output matrix must have one row per point and one column per line element"
        );

        self.free_wake_per_line_element(points, out);
    }

    #[cfg(not(feature = "parallel"))]
    /// Serial version. The panels are the outer loop, to reuse the panel data for all points.
    fn free_wake_per_line_element(&self, points: &[SpatialVector], out: &mut Matrix<SpatialVector>) {
        let nr_cols = self.indices.nr_panels_along_span;

        out.data.fill(SpatialVector::default());

        for stream_index in 1..self.indices.nr_panels_per_line_element {
            for span_index in 0..nr_cols {
                let panel_index = self.indices.panel_index(stream_index, span_index);
                let strength = self.strengths[panel_index];

                if strength != 0.0 {
                    for (point_index, &point) in points.iter().enumerate() {
                        out.data[point_index * nr_cols + span_index] += strength *
                            self.unit_strength_induced_velocity_from_panel_flat_index(panel_index, point);
                    }
                }
            }
        }
    }

    #[cfg(feature = "parallel")]
    /// Parallel version. Each point, i.e., each row in the output, is computed in parallel.
    ///
    /// The panels are visited in the order they are stored, which is important for the
    /// performance, as the panel data is then read sequentially from memory.
    fn free_wake_per_line_element(&self, points: &[SpatialVector], out: &mut Matrix<SpatialVector>) {
        let nr_cols = self.indices.nr_panels_along_span;
        let nr_panels = self.strengths.len();

        out.data.par_chunks_mut(nr_cols)
            .zip(points.par_iter())
            .for_each(|(row, &point)| {
                row.fill(SpatialVector::default());

                for row_start in (nr_cols..nr_panels).step_by(nr_cols) {
                    let strengths = &self.strengths[row_start..row_start + nr_cols];

                    for (span_index, (value, &strength)) in row.iter_mut().zip(strengths).enumerate() {
                        if strength != 0.0 {
                            *value += strength * self.unit_strength_induced_velocity_from_panel_flat_index(
                                row_start + span_index,
                                point
                            );
                        }
                    }
                }
            });
    }

    #[cfg(not(feature = "parallel"))]
    /// Calculates the induced velocities from the panels starting at start_index and ending at
    /// end_index
    fn induced_velocities_from_panel_range(
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

    #[cfg(feature = "parallel")]
    /// Calculates the induced velocities from the panels starting at start_index and ending at
    /// end_index
    fn induced_velocities_from_panel_range(
        &self,
        points: &[SpatialVector],
        start_index: usize,
        end_index: usize
    ) -> Vec<SpatialVector> {
        // Pre-collect active panels once (avoids repeated zero-checks per point)
        let active_panels: Vec<(usize, Float)> = (start_index..end_index)
            .filter_map(|i| {
                let s = self.strengths[i];
                if s != 0.0 { Some((i, s)) } else { None }
            })
            .collect();

        points.par_iter()
            .map(|&point| {
                active_panels.iter()
                    .map(|&(panel_index, strength)| {
                        strength *
                        self.unit_strength_induced_velocity_from_panel_flat_index(
                            panel_index,
                            point
                        )
                    })
                    .fold(SpatialVector::default(), |acc, v| acc + v)
            })
            .collect()
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
}
