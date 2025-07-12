
use stormath::spatial_vector::SpatialVector;
use crate::line_force_model::span_line::SpanLine;
use crate::lifting_line::singularity_elements::panel::Panel;

use stormath::array2::Array2;

use rayon::prelude::*;

use crate::lifting_line::wake::Wake;
#[derive(Debug, Clone)]
/// Represents a wake where the shape is assumed to be frozen, but where the strength on parts of 
/// the wake can be updated. That is, it is intended to be used while solving for the circulation
/// strength in a lifting line simulation. Two primary scenarios exists:
/// 
/// - The wake is steady, and consists of just one panel per span line, where the strength is
/// unknown
/// - The wake is actually dynamic, but most of the wake consists of panels where the strength is 
/// known from previous time steps. The only unknown strength is the strength of the first panels 
/// right behind the span lines making up the wings. The induced velocities therefore comes from 
/// both the panels with known strength and the panels with unknown strength.
pub struct FrozenWake {
    /// Vector containing values for the induced velocities that are constant for each control point 
    /// in the simulation. That is, velocities that do not depend on the circulation strength of the
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
    pub fn initialize(nr_span_lines: usize) -> Self {
        let fixed_velocities = vec![SpatialVector::<3>::default(); nr_span_lines];

        let variable_velocity_factors = Array2::new_default(
            [nr_span_lines, nr_span_lines]
        );

        FrozenWake {
            fixed_velocities,
            variable_velocity_factors,
        }
    }

    /// Function to create a steady frozen wake from a set of span lines, a wake direction and a
    /// wake length.
    pub fn steady_wake_from_span_lines_and_direction(
        span_lines: &[SpanLine],
        wake_vector: SpatialVector<3>,
        viscous_core_length: f64,
        far_field_ratio: f64,
    ) -> Self {
        let nr_span_lines = span_lines.len();

        let ctrl_points: Vec<SpatialVector<3>> = span_lines.iter().map(
            |span_line| span_line.ctrl_point()
        ).collect();

        let fixed_velocities = vec![SpatialVector::<3>::default(); nr_span_lines];
        let mut variable_velocity_factors = Array2::new_default(
            [nr_span_lines, nr_span_lines]
        );

        let mut panels: Vec<Panel> = Vec::with_capacity(nr_span_lines);

        for i in 0..nr_span_lines {
            let points = [
                span_lines[i].start_point + wake_vector,
                span_lines[i].start_point,
                span_lines[i].end_point,
                span_lines[i].end_point + wake_vector,
            ];
            let panel = Panel::new(
                points,
                far_field_ratio, 
                viscous_core_length,
            );
            panels.push(panel);
        }

        for row_index in 0..nr_span_lines {
            let ctrl_point = ctrl_points[row_index];

            for col_index in 0..nr_span_lines {
                let panel = &panels[col_index];

                let induced_velocity = panel.induced_velocity_with_unit_strength(ctrl_point);

                variable_velocity_factors[[row_index, col_index]] = induced_velocity;
            }
        }

        FrozenWake {
            fixed_velocities,
            variable_velocity_factors,
        }

    }

    pub fn update(&mut self, wake: &Wake) {
        self.update_fixed_velocities(wake);
        self.update_variable_velocity_factors(wake);
    }

    pub fn update_fixed_velocities(&mut self, wake: &Wake) {
        let ctrl_points = wake.ctrl_points();

        self.fixed_velocities = wake.induced_velocities_from_free_wake(&ctrl_points);
    }

    pub fn update_variable_velocity_factors(&mut self, wake: &Wake) {
        let ctrl_points = wake.ctrl_points();

        let indices_logic = self.variable_velocity_factors.indices.clone();

        if wake.settings.neglect_self_induced_velocities {
            self.variable_velocity_factors.data.par_iter_mut().enumerate().for_each(|(flat_index, factor)| {
                let indices = indices_logic.indices_from_index(flat_index);

                let ctrl_point_index = indices[0];
                let panel_index = indices[1];
    
                let ctrl_point_wing_index = wake.wing_index(ctrl_point_index);
                let panel_wing_index      = wake.wing_index(panel_index);
    
                if ctrl_point_wing_index == panel_wing_index {
                    *factor = SpatialVector::<3>::default();
                } else {
                    *factor = wake.unit_strength_induced_velocity_from_panel(
                        0, 
                        panel_index, 
                        ctrl_points[ctrl_point_index]
                    );
                }
            });
        } else {
            self.variable_velocity_factors.data.par_iter_mut().enumerate().for_each(|(flat_index, factor)| {
                let indices = indices_logic.indices_from_index(flat_index);

                let ctrl_point_index = indices[0];
                let panel_index = indices[1];
    
                *factor = wake.unit_strength_induced_velocity_from_panel(
                    0, 
                    panel_index, 
                    ctrl_points[ctrl_point_index]
                );
            });
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
        self.fixed_velocities.iter().enumerate().map(
            |(i_row, u_fixed)| {
                let mut induced_velocity = *u_fixed;

                for i_col in 0..self.variable_velocity_factors.indices.shape[1] {
                    induced_velocity += 
                        self.variable_velocity_factors[[i_row, i_col]] * circulation_strength[i_col];
                }

                if induced_velocity[0].is_nan() || induced_velocity[1].is_nan() || induced_velocity[2].is_nan() {
                    induced_velocity = SpatialVector::<3>::default();
                }

                induced_velocity
            }
        ).collect()
    }
}