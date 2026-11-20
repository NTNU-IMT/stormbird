// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Correction of the sampled velocity in actuator line simulations, based on lifting line models.

/// Builders for the correction and its wake models
pub mod builder;
/// Straight wake, with one horseshoe vortex for each line element
pub mod straight;
/// Straight wake, split into panels in the streamwise direction
pub mod straight_panels;
/// Free wake, where the shape follows the velocity sampled from the CFD flow field
pub mod dynamic;
/// Functionality shared by the wake models that consist of vortex panels
pub mod panel_wake;

#[cfg(test)]
mod tests;

pub use builder::{
    LiftingLineCorrectionBuilder,
    CorrectionWakeBuilder,
    StraightWakeBuilder,
    StraightPanelWakeBuilder,
    DynamicCorrectionWakeBuilder,
};

use crate::lifting_line::singularity_elements::horseshoe_vortex::HorseshoeVortex;
use crate::lifting_line::singularity_elements::symmetry_condition::SymmetryCondition;

use crate::line_force_model::LineForceModel;

use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;
use stormath::transition_functions::cosine_transition_zero_to_one;

use straight::StraightWake;
use straight_panels::StraightPanelWake;
use dynamic::DynamicCorrectionWake;

#[derive(Debug, Clone)]
/// Settings that are shared by all wake models in the correction
pub struct CorrectionSettings {
    /// Viscous core length equal to the force projection width
    pub viscous_core_length: Float,
    /// Small reference viscous core length
    pub reference_core_length: Float,
    /// Optional symmetry condition
    pub symmetry_condition: SymmetryCondition,
    /// If true, the induced velocities from all wings are included in the correction
    pub include_wing_interactions: bool,
}

impl CorrectionSettings {
    /// Induced velocity from a horseshoe vortex with unit strength, including the effect of the
    /// symmetry condition.
    pub fn unit_strength_induced_velocity(
        &self,
        vortex: &HorseshoeVortex,
        point: SpatialVector,
    ) -> SpatialVector {
        let u_i = vortex.induced_velocity_with_unit_strength(point);

        if let Some(point_mirrored) = self.symmetry_condition.mirrored_point(point) {
            let u_i_mirrored = vortex.induced_velocity_with_unit_strength(point_mirrored);

            self.symmetry_condition.corrected_velocity(u_i, u_i_mirrored)
        } else {
            u_i
        }
    }
}

#[derive(Debug, Clone)]
// There is only one instance of the enum in each actuator line model, so the size difference
// between the variants does not matter
#[allow(clippy::large_enum_variant)]
/// The wake model used to compute the correction
pub enum CorrectionWake {
    /// One straight horseshoe vortex for each line element
    Straight(StraightWake),
    /// Straight wake, split into panels in the streamwise direction
    StraightPanels(StraightPanelWake),
    /// Free wake, where the shape follows the velocity sampled from the CFD flow field
    Dynamic(DynamicCorrectionWake),
}

#[derive(Debug, Clone)]
/// A structure used to compute corrections for the velocity, based on a lifting line model. The
/// model computes two versions of induced velocities using a lifting line representation; one where
/// the viscous core length is set to a small value, and one where it is set to a value equal to the
/// force projection width. The difference in induced velocities are then added to the velocity that
/// is sampled directly from the flow field. This is to account for the fact that realistic force
/// projection widths tend to smear out the tip vortices, leading to under predicted induced
/// velocities.
///
/// By default, the correction is computed for each wing in isolation, so that only errors in the
/// self-induced velocities are corrected. If `include_wing_interactions` is true, the induced
/// velocities from all wings are included, so that errors in the wing-to-wing interactions are
/// also corrected.
///
/// The shape of the wake is given by the [CorrectionWake]. Each time step, or each call to the
/// solver, the wake is first updated with [`update`](Self::update), and the correction is then
/// computed with [`velocity_correction`](Self::velocity_correction).
pub struct LiftingLineCorrection {
    /// Settings that are shared by all wake models
    pub settings: CorrectionSettings,
    /// Optional time over which the correction is gradually increased from zero
    pub initialization_time: Option<Float>,
    /// Optional limit on the magnitude of the velocity correction, as a ratio of the magnitude of
    /// the sampled velocity
    pub max_correction_ratio: Option<Float>,
    /// The wake model
    pub wake: CorrectionWake,
}

impl LiftingLineCorrection {
    /// Updates the wake model before the correction is computed, based on the current state of the
    /// line force model and the sampled velocity at the control points. Does nothing for the
    /// straight wake, which is rebuilt in each call to
    /// [`velocity_correction`](Self::velocity_correction).
    ///
    /// The circulation from the previous time step, the time and the time step are only used by the
    /// dynamic wake, which is updated once per time step.
    #[allow(clippy::too_many_arguments)]
    pub fn update(
        &mut self,
        line_force_model: &LineForceModel,
        ctrl_points_velocity: &[SpatialVector],
        previous_reference_circulation_strength: &[Float],
        previous_projected_circulation_strength: &[Float],
        time: Float,
        time_step: Float,
    ) {
        match &mut self.wake {
            CorrectionWake::Straight(_) => {},
            CorrectionWake::StraightPanels(wake) => {
                wake.update_geometry(&self.settings, line_force_model, ctrl_points_velocity);
            },
            CorrectionWake::Dynamic(wake) => {
                wake.update(
                    &self.settings,
                    line_force_model,
                    ctrl_points_velocity,
                    previous_reference_circulation_strength,
                    previous_projected_circulation_strength,
                    time,
                    time_step
                );
            },
        }
    }

    /// The points in the wake where the velocity must be sampled from the CFD flow field. Empty
    /// unless the wake is dynamic, and before the first call to [`update`](Self::update).
    pub fn wake_points_to_sample(&self) -> &[SpatialVector] {
        match &self.wake {
            CorrectionWake::Dynamic(wake) => wake.points_to_sample(),
            _ => &[],
        }
    }

    /// The wake as a legacy VTK PolyData buffer, with the geometry, the strength and the core
    /// length of the panels in both wake models, and the sampled velocity at the points for the
    /// dynamic wake. The rows of panels that use the current circulation in the correction get the
    /// given circulation. None for the straight wake, which has no panels, and before the first
    /// call to [`update`](Self::update).
    pub fn wake_as_vtk(
        &self,
        reference_circulation_strength: &[Float],
        projected_circulation_strength: &[Float],
        binary: bool,
    ) -> Option<Vec<u8>> {
        match &self.wake {
            CorrectionWake::Straight(_) => None,
            CorrectionWake::StraightPanels(wake) => wake.as_vtk(
                reference_circulation_strength, projected_circulation_strength, binary
            ),
            CorrectionWake::Dynamic(wake) => wake.as_vtk(
                reference_circulation_strength, projected_circulation_strength, binary
            ),
        }
    }

    /// Sets the sampled velocity at the wake point with the given index, see
    /// [`wake_points_to_sample`](Self::wake_points_to_sample). Values that are not finite are
    /// ignored, so that the previous value is kept.
    ///
    /// # Panics
    /// If the wake is not dynamic, or the index is out of bounds.
    pub fn set_wake_point_velocity(&mut self, index: usize, velocity: SpatialVector) {
        match &mut self.wake {
            CorrectionWake::Dynamic(wake) => wake.set_wake_point_velocity(index, velocity),
            _ => panic!("Wake point velocities can only be set for the dynamic correction wake"),
        }
    }

    /// Computes a difference in the induced velocities between two lifting line models; one with a
    /// small viscous core length, and one with a viscous core length equal to the force projection
    /// width. The difference is returned as a vector of velocities at the control points of the
    /// line force model.
    ///
    /// The model with the viscous core represents the velocities that are induced in the CFD
    /// domain, and therefore uses the circulation that the projected forces are computed from. The
    /// reference model, including the virtual extensions, uses the circulation that the forces on
    /// the wings are computed from. The two are equal, unless a tip loss correction is only applied
    /// to the projected forces.
    ///
    /// The wake must be updated with [`update`](Self::update) first.
    pub fn velocity_correction(
        &self,
        line_force_model: &LineForceModel,
        ctrl_points_velocity: &[SpatialVector],
        reference_circulation_strength: &[Float],
        projected_circulation_strength: &[Float],
        time: Float,
    ) -> Vec<SpatialVector> {
        let mut u_i_correction = match &self.wake {
            CorrectionWake::Straight(wake) => wake.induced_velocity_difference(
                &self.settings,
                line_force_model,
                ctrl_points_velocity,
                reference_circulation_strength,
                projected_circulation_strength
            ),
            CorrectionWake::StraightPanels(wake) => wake.induced_velocity_difference(
                &self.settings,
                line_force_model,
                reference_circulation_strength,
                projected_circulation_strength
            ),
            CorrectionWake::Dynamic(wake) => wake.induced_velocity_difference(
                &self.settings,
                line_force_model,
                reference_circulation_strength,
                projected_circulation_strength
            ),
        };

        if let Some(initialization_time) = self.initialization_time {
            let correction_factor = cosine_transition_zero_to_one(time, 0.0, initialization_time);

            for u_i in u_i_correction.iter_mut() {
                *u_i *= correction_factor;
            }
        }

        // The correction can become larger than the sampled velocity close to the ends of the
        // wings, where the reference model with the small viscous core is close to singular. The
        // limit reduces the magnitude of the correction, but keeps its direction.
        if let Some(max_correction_ratio) = self.max_correction_ratio {
            for (u_i, velocity) in u_i_correction.iter_mut().zip(ctrl_points_velocity) {
                let max_magnitude = max_correction_ratio * velocity.length();
                let magnitude = u_i.length();

                if magnitude > max_magnitude {
                    *u_i *= max_magnitude / magnitude;
                }
            }
        }

        u_i_correction
    }
}
