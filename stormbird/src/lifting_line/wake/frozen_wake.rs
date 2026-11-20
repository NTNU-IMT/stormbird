// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use std::ops::Range;

use stormath::spatial_vector::SpatialVector;
use crate::line_force_model::span_line::SpanLine;
use crate::lifting_line::singularity_elements::horseshoe_vortex::HorseshoeVortex;
use crate::lifting_line::singularity_elements::symmetry_condition::SymmetryCondition;
use crate::lifting_line::wake::quasi_steady_wake::QuasiSteadyWake;

use stormath::matrix::Matrix;
use stormath::type_aliases::Float;

use crate::lifting_line::wake::dynamic_wake::DynamicWake;

#[derive(Debug, Clone)]
/// Induced velocities at the control points of a line force model, split by where they come from.
pub struct InducedVelocities {
    /// The velocities induced by the wake of the same wing as the control point
    pub self_induced: Vec<SpatialVector>,
    /// The velocities induced by the wakes of all the other wings
    pub other_induced: Vec<SpatialVector>,
}

impl InducedVelocities {
    /// Creates a new structure with all velocities set to zero, for the given number of control
    /// points
    pub fn new(nr_ctrl_points: usize) -> Self {
        Self {
            self_induced: vec![SpatialVector::default(); nr_ctrl_points],
            other_induced: vec![SpatialVector::default(); nr_ctrl_points],
        }
    }

    /// The number of control points
    pub fn len(&self) -> usize {
        self.self_induced.len()
    }

    /// Returns true if there are no control points
    pub fn is_empty(&self) -> bool {
        self.self_induced.is_empty()
    }

    /// The total induced velocity at each control point, i.e., the sum of the self-induced and the
    /// other-induced velocities
    pub fn total(&self) -> Vec<SpatialVector> {
        self.self_induced.iter().zip(&self.other_induced)
            .map(|(&self_induced, &other_induced)| self_induced + other_induced)
            .collect()
    }
}

#[derive(Debug, Clone)]
/// Represents a wake where the shape is assumed to be frozen, but where the strength on parts of
/// the wake can be updated. That is, it is intended to be used while solving for the circulation
/// strength in a lifting line simulation. Two primary scenarios exists:
///
/// - The wake is steady, and consists of just one panel/horseshoe vortex per span line, where the
///   strength is unknown
/// - The wake is actually dynamic, but most of the wake consists of panels where the strength is
///   known from previous time steps. The only unknown strength is the strength of the first panels
///   right behind the span lines making up the wings. The induced velocities therefore comes from
///   both the panels with known strength and the panels with unknown strength.
///
/// The induced velocities are always computed as two parts: the velocities induced by the wake of
/// the same wing as the control point (self-induced), and the velocities induced by the wakes of
/// the other wings (other-induced). The line elements, and therefore also the control points and
/// the columns of the velocity factors, are ordered wing by wing, so that each wing is a contiguous
/// range given by `wing_indices`. The split is therefore done with blocks of the matrices, without
/// any checks of the wing index for each element.
pub struct FrozenWake {
    /// The range of line elements that belong to each wing. Copied from the line force model.
    pub wing_indices: Vec<Range<usize>>,
    /// Induced velocities at each control point that are constant while solving for the
    /// circulation strength. That is, velocities that do not depend on the circulation strength of
    /// the panels right behind the line model.
    pub fixed_velocities: InducedVelocities,
    /// Matrix containing coefficients that can be used to calculate induced velocities as a
    /// function of the strength of each vortex line.
    ///
    /// The shape of the matrix is (nr_span_lines, nr_span_lines). Each row corresponds to a control
    /// point. Each column for a given row corresponds to the induced velocity from each panel. The
    /// induced velocity can therefore be calculated as the dot product of the row and the
    /// circulation strength.
    pub variable_velocity_factors: Matrix<SpatialVector>,
    /// Allocate space for the induced velocities at the control points, to avoid reallocating
    /// every time step.
    pub induced_velocities_at_control_points: InducedVelocities,
    /// Buffer for the velocities induced by the free part of a dynamic wake, for each control point
    /// (row) and each line element that the panels are shed from (column). Reused every time step
    /// to avoid reallocation.
    fixed_velocities_per_line_element: Matrix<SpatialVector>,
}

impl FrozenWake {
    /// Creates a frozen wake with all velocities set to zero, for a line force model where the
    /// line elements of each wing are given by the `wing_indices`
    pub fn initialize(wing_indices: Vec<Range<usize>>) -> Self {
        let nr_span_lines = wing_indices.last().map(|range| range.end).unwrap_or(0);

        Self::check_wing_indices(&wing_indices);

        FrozenWake {
            wing_indices,
            fixed_velocities: InducedVelocities::new(nr_span_lines),
            variable_velocity_factors: Matrix::new_default([nr_span_lines, nr_span_lines]),
            induced_velocities_at_control_points: InducedVelocities::new(nr_span_lines),
            fixed_velocities_per_line_element: Matrix::new_default([nr_span_lines, nr_span_lines]),
        }
    }

    /// The block structure assumes that the wings are contiguous, ordered and cover all the line
    /// elements.
    fn check_wing_indices(wing_indices: &[Range<usize>]) {
        let mut expected_start = 0;

        for range in wing_indices {
            assert!(
                range.start == expected_start && range.end >= range.start,
                "The wing indices in the frozen wake must be contiguous and ordered, but got {:?}",
                wing_indices
            );

            expected_start = range.end;
        }
    }

    /// Number of control points, which is also the number of line elements
    pub fn nr_ctrl_points(&self) -> usize {
        self.variable_velocity_factors.shape[0]
    }

    /// Updates the variable velocity factors from a quasi-steady wake, where the geometry of the
    /// horseshoe vortices is already updated for the current time step.
    ///
    /// The whole horseshoe vortex has the unknown strength, so the fixed velocities are zero.
    pub fn update_from_quasi_steady_wake(
        &mut self,
        ctrl_points: &[SpatialVector],
        wake: &QuasiSteadyWake,
    ) {
        let nr_span_lines = ctrl_points.len();

        for (row_index, &ctrl_point) in ctrl_points.iter().enumerate().take(nr_span_lines) {

            for col_index in 0..nr_span_lines {
                self.variable_velocity_factors[[row_index, col_index]] =
                    wake.unit_strength_induced_velocity(col_index, ctrl_point);
            }
        }
    }

    /// Function to create a steady frozen wake from a set of span lines and velocity vectors for
    /// each end points of the span lines. All span lines are assumed to belong to the same wing.
    pub fn new_for_single_wing_from_span_lines_and_velocities(
        span_lines: &[SpanLine],
        chord_lengths: &[Float],
        velocities: &[SpatialVector],
        wake_length: Float,
        viscous_core_length: Float,
        symmetry_condition: SymmetryCondition
    ) -> Self {
        let nr_span_lines = span_lines.len();

        // One wing, covering all the line elements
        #[allow(clippy::single_range_in_vec_init)]
        let mut frozen_wake = Self::initialize(vec![0..nr_span_lines]);

        let ctrl_points: Vec<SpatialVector> = span_lines.iter().map(
            |span_line| span_line.ctrl_point()
        ).collect();

        let wake_vectors: Vec<SpatialVector> = velocities.iter().map(|v| {
            wake_length * v.normalize()
        }).collect();

        let horseshoe_vortices = HorseshoeVortex::vortices_for_single_wing_from_span_lines_and_wake_vectors(
            span_lines,
            chord_lengths,
            &wake_vectors,
            viscous_core_length
        );

        for (row_index, &ctrl_point) in ctrl_points.iter().enumerate().take(nr_span_lines) {

            for col_index in 0..nr_span_lines {
                let vortex = &horseshoe_vortices[col_index];

                let u_i = vortex.induced_velocity_with_unit_strength(ctrl_point);

                let point_mirrored = symmetry_condition.mirrored_point(ctrl_point);

                let u_i_corrected = if let Some(p_m) = point_mirrored {
                    let u_i_mirrored = vortex.induced_velocity_with_unit_strength(p_m);
                    symmetry_condition.corrected_velocity(u_i, u_i_mirrored)
                } else {
                    u_i
                };

                frozen_wake.variable_velocity_factors[[row_index, col_index]] = u_i_corrected;
            }
        }

        frozen_wake
    }

    /// Function to create a steady frozen wake from a set of span lines, a wake direction and a
    /// wake length.
    pub fn new_for_single_wing_from_span_lines_and_direction(
        span_lines: &[SpanLine],
        chord_lengths: &[Float],
        wake_vector: SpatialVector,
        viscous_core_length: Float,
        symmetry_condition: SymmetryCondition
    ) -> Self {
        let nr_span_lines = span_lines.len();

        let velocities = vec![wake_vector.normalize(); nr_span_lines + 1];

        Self::new_for_single_wing_from_span_lines_and_velocities(
            span_lines,
            chord_lengths,
            &velocities,
            wake_vector.length(),
            viscous_core_length,
            symmetry_condition
        )
    }

    /// Updates the fixed velocities and the velocity factors from the dynamic wake
    pub fn update_from_full_wake(&mut self, ctrl_points: &[SpatialVector], wake: &DynamicWake) {
        self.update_fixed_velocities(ctrl_points, wake);
        self.update_variable_velocity_factors(ctrl_points, wake);
    }

    /// Updates the velocities induced by the free part of the dynamic wake, which do not change while
    /// solving for the circulation strength. Not updated with the steady state strength update, as
    /// the whole wake then has the unknown strength.
    ///
    /// The wake computes the velocity from the panels shed from each line element separately, and
    /// these are then split into self-induced and other-induced velocities with the same block
    /// structure as the variable velocity factors.
    pub fn update_fixed_velocities(&mut self, ctrl_points: &[SpatialVector], wake: &DynamicWake) {
        if wake.settings.steady_state_strength_update {
            return;
        }

        wake.induced_velocities_from_free_wake_per_line_element(
            ctrl_points,
            &mut self.fixed_velocities_per_line_element
        );

        let nr_cols = self.fixed_velocities_per_line_element.shape[1];

        for wing_range in &self.wing_indices {
            for i_row in wing_range.clone() {
                let row_start = i_row * nr_cols;
                let row = &self.fixed_velocities_per_line_element.data[row_start..row_start + nr_cols];

                self.fixed_velocities.self_induced[i_row] = sum_slice(&row[wing_range.clone()]);

                self.fixed_velocities.other_induced[i_row] =
                    sum_slice(&row[..wing_range.start]) +
                    sum_slice(&row[wing_range.end..]);
            }
        }
    }

    #[inline(always)]
    fn induced_velocity_stream_strip(
        ctrl_point: SpatialVector,
        panel_index: usize,
        stream_length: usize,
        wake: &DynamicWake
    ) -> SpatialVector {
        (0..stream_length).map(|stream_index| {
            wake.unit_strength_induced_velocity_from_panel(
                stream_index,
                panel_index,
                ctrl_point
            )
        }).sum()
    }

    /// Updates the velocity factors from the parts of the dynamic wake that depend on the circulation
    /// strength on the wings
    pub fn update_variable_velocity_factors(
        &mut self,
        ctrl_points: &[SpatialVector],
        wake: &DynamicWake
    ) {
        let [nr_ctrl_points, nr_panels] = self.variable_velocity_factors.shape;

        let stream_length = if wake.settings.steady_state_strength_update {
            wake.indices.nr_panels_per_line_element
        } else {
            1
        };

        for panel_index in 0..nr_panels {
            for (ctrl_point_index, &ctrl_point) in ctrl_points.iter().enumerate().take(nr_ctrl_points) {

                let induced_velocity = Self::induced_velocity_stream_strip(
                    ctrl_point,
                    panel_index,
                    stream_length,
                    wake
                );

                self.variable_velocity_factors[[ctrl_point_index, panel_index]] = induced_velocity;
            }
        }
    }

    /// Update the stored induced velocity at the control points, given the circulation strength.
    ///
    /// For each control point, the row of velocity factors is split in three contiguous parts: the
    /// line elements before the wing of the control point, the line elements of the wing itself,
    /// and the line elements after the wing. The first and the last part give the other-induced
    /// velocities, and the middle part the self-induced velocities. The number of operations is the
    /// same as for computing the total velocity directly.
    ///
    /// # Arguments
    /// * `circulation_strength` - the circulation strength of the span lines that make up the
    ///   wake. The strength is assumed to be constant along the length of the span line.
    pub fn update_induced_velocities_at_control_points(
        &mut self,
        circulation_strength: &[Float],
    ) {
        let nr_cols = self.variable_velocity_factors.shape[1];
        let strength = &circulation_strength[..nr_cols];

        for wing_range in &self.wing_indices {
            let (start, end) = (wing_range.start, wing_range.end);

            for i_row in wing_range.clone() {
                let row_start = i_row * nr_cols;
                let row = &self.variable_velocity_factors.data[row_start..row_start + nr_cols];

                self.induced_velocities_at_control_points.self_induced[i_row] =
                    self.fixed_velocities.self_induced[i_row] +
                    dot_slice(&row[start..end], &strength[start..end]);

                self.induced_velocities_at_control_points.other_induced[i_row] =
                    self.fixed_velocities.other_induced[i_row] +
                    dot_slice(&row[..start], &strength[..start]) +
                    dot_slice(&row[end..], &strength[end..]);
            }
        }
    }
}

#[inline(always)]
/// Dot product between a slice of velocity factors and a slice of strengths. The components are
/// summed separately to help auto-vectorization.
fn dot_slice(factors: &[SpatialVector], strengths: &[Float]) -> SpatialVector {
    let mut sum_x = 0.0;
    let mut sum_y = 0.0;
    let mut sum_z = 0.0;

    for (factor, &strength) in factors.iter().zip(strengths) {
        sum_x += factor.0[0] * strength;
        sum_y += factor.0[1] * strength;
        sum_z += factor.0[2] * strength;
    }

    SpatialVector::from([sum_x, sum_y, sum_z])
}

#[inline(always)]
/// Sum of a slice of vectors, with the components summed separately as in [`dot_slice`]
fn sum_slice(vectors: &[SpatialVector]) -> SpatialVector {
    let mut sum_x = 0.0;
    let mut sum_y = 0.0;
    let mut sum_z = 0.0;

    for vector in vectors {
        sum_x += vector.0[0];
        sum_y += vector.0[1];
        sum_z += vector.0[2];
    }

    SpatialVector::from([sum_x, sum_y, sum_z])
}
