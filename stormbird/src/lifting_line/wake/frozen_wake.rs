
use math_utils::spatial_vector::SpatialVector;

use ndarray::prelude::*;

use rayon::prelude::*;

use crate::lifting_line::wake::unsteady::UnsteadyWake;
use crate::line_force_model::LineForceModel;

#[derive(Debug, Clone)]
/// Represents a wake where the shape is assumed to be frozen, but where the strength on parts of 
/// the wake can be updated. That is, it is intended to be used while solving for the circulation
/// strength in a lifting line simulation. Two primary scenarios exists:
/// 
/// - The wake is steady, and consists of one horseshoe vortex per span line, where the strength is
/// unknown
/// - The wake is actually dynamic, but most of the wake consists of panels where the strength is 
/// known from previous time steps. The only unknown strength is the strength of the first panels 
/// right behind the span lines making up the wings. The induced velocities therefore comes from 
/// both the panels with known strength and the panels with unknown strength.
pub struct FrozenWake {
    /// Vector containing values for the induced velocities that are constant each control point in 
    /// the simulation. That is, velocities that do not depend on the circulation strength of the
    /// panels right behind the line model.
    pub fixed_velocities: Vec<SpatialVector<3>>,
    /// Matrix containing coefficients that can be used to calculate induced velocities as a 
    /// function of the strength of each vortex line. 
    /// 
    /// The shape of the matrix is (nr_span_lines, nr_span_lines). Each row corresponds to a control
    /// point. Each column for a given row corresponds to the induced velocity from each panel. The 
    /// induced velocity can therefore be calculated as the dot product of the row and the 
    /// circulation strength.
    pub variable_velocity_factors: Array2<SpatialVector<3>>,
}

impl FrozenWake {
    /// Construct a frozen wake from a full dynamic wake. 
    pub fn from_wake(line_force_model: &LineForceModel, wake: &mut UnsteadyWake) -> Self {
        let ctrl_points = line_force_model.ctrl_points();

        let nr_span_lines = ctrl_points.len();

        let fixed_velocities = wake.induced_velocities_from_free_wake(&ctrl_points, false);

        let mut variable_velocity_factors: Array2<SpatialVector<3>> = Array2::from_elem(
            (nr_span_lines, nr_span_lines), SpatialVector::<3>::default()
        );

        for ctrl_point_index in 0..nr_span_lines {
            for panel_index in 0..nr_span_lines {
                variable_velocity_factors[[ctrl_point_index, panel_index]] = 
                    wake.unit_strength_induced_velocity_from_panel(
                        0, 
                        ctrl_point_index, 
                        ctrl_points[ctrl_point_index], 
                        false
                    );
            }
        }

        FrozenWake {
            fixed_velocities,
            variable_velocity_factors,
        }
    }

    /// Returns the total velocity at the control points, given the circulation strength.
    /// 
    /// # Arguments
    /// * `circulation_strength` - the circulation strength of the span lines that make up the 
    /// wake. The strength is assumed to be constant along the length of the span line.
    pub fn induced_velocities_at_control_points(
        &self,
        circulation_strength: &[f64],
    ) -> Vec<SpatialVector<3>> {
        let mut induced_velocities = vec![SpatialVector::<3>::default(); self.fixed_velocities.len()];

        induced_velocities.par_iter_mut().enumerate().for_each(|(i_row, induced_velocity)| {
            *induced_velocity = self.fixed_velocities[i_row];

            for i_col in 0..self.variable_velocity_factors.shape()[1] {
                *induced_velocity += 
                    self.variable_velocity_factors[[i_row, i_col]] * circulation_strength[i_col];
            }
        });

        induced_velocities
    }
}