// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! The dynamic wake model of the lifting line correction, where the shape of the wake follows the
//! velocity sampled from the CFD flow field.

use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

use crate::lifting_line::wake::dynamic_wake::builder::DynamicWakeBuilder;
use crate::lifting_line::wake::dynamic_wake::settings::FirstWakePointsDirection;
use crate::line_force_model::LineForceModel;
use crate::error::Error;

use super::CorrectionSettings;
use super::builder::DynamicCorrectionWakeBuilder;
use super::panel_wake::PanelWakePair;

#[derive(Debug, Clone)]
/// A free wake, where the shape follows the velocity sampled from the CFD flow field, and the
/// strength of each row of panels is the circulation from earlier time steps.
///
/// The wake moves with the sampled CFD velocity only. The sampled velocity already contains the
/// velocities induced by the actuator line, so no induced velocities are computed from the wakes
/// themselves. The first free row of points follows the direction of the sampled velocity at the
/// control points, with the spanwise component and the motion of the wing removed. The other
/// points move with the velocity at the points of the wake, which must be set by the CFD solver,
/// see [`set_wake_point_velocity`](Self::set_wake_point_velocity).
///
/// The wake is updated once per time step, in the first call to [`update`](Self::update) with a
/// new time. The points are moved with the Euler method: the velocity sampled at the positions
/// from the last update moves the points to the positions used in the new time step.
pub struct DynamicCorrectionWake {
    /// The panel wakes with the viscous and the reference core length. They have the same geometry
    pub wakes: PanelWakePair,
    /// Sampled velocity at the points of the wake
    pub wake_points_velocity: Vec<SpatialVector>,
    /// Time of the last update, to detect new time steps. None before the first update.
    last_update_time: Option<Float>,
}

impl DynamicCorrectionWake {
    /// Creates the wake from the builder. The geometry is set in the first call to
    /// [`update`](Self::update).
    pub fn new(
        builder: &DynamicCorrectionWakeBuilder,
        settings: &CorrectionSettings,
        line_force_model: &LineForceModel,
    ) -> Result<Self, Error> {
        let wake_builder = DynamicWakeBuilder {
            nr_panels_per_line_element: builder.nr_panels_per_line_element,
            first_panel_relative_length: builder.first_panel_relative_length,
            last_panel_relative_length: builder.last_panel_relative_length,
            first_wake_points_direction: FirstWakePointsDirection::Freestream,
            shape_damping_factor: builder.shape_damping_factor,
            far_field_ratio: builder.far_field_ratio,
            steady_state_strength_update: false,
            ratio_of_wake_affected_by_induced_velocities: 0.0,
            ..Default::default()
        };

        let wakes = PanelWakePair::new(
            wake_builder,
            settings,
            line_force_model,
            builder.reference_core_evolution_length_factor
        )?;

        let nr_points = wakes.reference.points.len();

        Ok(Self {
            wakes,
            wake_points_velocity: vec![SpatialVector::default(); nr_points],
            last_update_time: None,
        })
    }

    /// Whether the wake has been initialized in the first call to [`update`](Self::update)
    pub fn is_initialized(&self) -> bool {
        self.last_update_time.is_some()
    }

    /// The points where the velocity should be sampled from the CFD flow field. The two wakes have
    /// the same geometry, so the points of the reference wake are used. Empty before the first
    /// call to [`update`](Self::update), as the geometry is set there.
    pub fn points_to_sample(&self) -> &[SpatialVector] {
        if self.is_initialized() {
            &self.wakes.reference.points
        } else {
            &[]
        }
    }

    /// Sets the sampled velocity at a point of the wake. Values that are not finite, e.g., from
    /// points outside the CFD domain, are ignored, so that the previous value is kept.
    pub fn set_wake_point_velocity(&mut self, index: usize, velocity: SpatialVector) {
        if velocity.0.iter().all(|value| value.is_finite()) {
            self.wake_points_velocity[index] = velocity;
        }
    }

    /// Updates the wake at the start of a new time step. Does nothing if the time is the same as in
    /// the last update, e.g., in outer iterations of the CFD solver.
    ///
    /// In the first call, both wakes are initialized as straight wakes along the average sampled
    /// velocity at the control points, with zero strength. In later time steps, the circulation
    /// from the previous time step is stored in the first row of panels, the points are moved with
    /// the sampled velocity, and the strengths are streamed one row downstream.
    ///
    /// # Arguments
    /// * `previous_reference_circulation_strength` - The circulation that the forces on the wings
    ///   were computed from in the previous time step. Used in the reference wake.
    /// * `previous_projected_circulation_strength` - The circulation that the projected forces were
    ///   computed from in the previous time step. Used in the viscous wake.
    #[allow(clippy::too_many_arguments)]
    pub fn update(
        &mut self,
        settings: &CorrectionSettings,
        line_force_model: &LineForceModel,
        ctrl_points_velocity: &[SpatialVector],
        previous_reference_circulation_strength: &[Float],
        previous_projected_circulation_strength: &[Float],
        time: Float,
        time_step: Float,
    ) {
        match self.last_update_time {
            None => {
                let average_velocity = ctrl_points_velocity.iter().copied().sum::<SpatialVector>() /
                    ctrl_points_velocity.len() as Float;

                for wake in [&mut self.wakes.viscous, &mut self.wakes.reference] {
                    wake.initialize_with_velocity_and_time_step(
                        line_force_model,
                        average_velocity,
                        time_step
                    );
                }

                self.wake_points_velocity = vec![average_velocity; self.wake_points_velocity.len()];
            },
            Some(last_update_time) if time > last_update_time => {
                let wakes_and_strengths = [
                    (&mut self.wakes.viscous, previous_projected_circulation_strength),
                    (&mut self.wakes.reference, previous_reference_circulation_strength),
                ];

                for (wake, previous_circulation_strength) in wakes_and_strengths {
                    wake.update_after_solving(
                        previous_circulation_strength,
                        &self.wake_points_velocity
                    );

                    wake.update_wake_points_before_solving(
                        time_step,
                        line_force_model,
                        ctrl_points_velocity
                    );

                    wake.stream_strength_values_downstream();
                }
            },
            Some(_) => return,
        }

        self.wakes.update_panels(settings);

        self.last_update_time = Some(time);
    }

    /// The wake as a legacy VTK PolyData buffer, see [`PanelWakePair::as_vtk`]. The first row of
    /// panels has the given circulation, and the sampled velocity at the wake points is included as
    /// point data. None before the first update.
    pub fn as_vtk(
        &self,
        reference_circulation_strength: &[Float],
        projected_circulation_strength: &[Float],
        binary: bool,
    ) -> Option<Vec<u8>> {
        if !self.is_initialized() {
            return None;
        }

        Some(self.wakes.as_vtk(
            reference_circulation_strength,
            projected_circulation_strength,
            1,
            Some(&self.wake_points_velocity),
            binary
        ))
    }

    /// The difference between the induced velocities from the reference wake and the viscous wake,
    /// at the control points of the line force model. The first row of panels gets the given
    /// circulation, while the rest use the circulation from earlier time steps.
    pub fn induced_velocity_difference(
        &self,
        settings: &CorrectionSettings,
        line_force_model: &LineForceModel,
        reference_circulation_strength: &[Float],
        projected_circulation_strength: &[Float],
    ) -> Vec<SpatialVector> {
        assert!(
            self.is_initialized(),
            "The dynamic correction wake must be updated before the correction is computed"
        );

        self.wakes.induced_velocity_difference(
            settings,
            line_force_model,
            reference_circulation_strength,
            projected_circulation_strength,
            1
        )
    }
}
