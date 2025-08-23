use serde::{Deserialize, Serialize};

use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

use crate::line_force_model::LineForceModel;
use crate::common_utils::forces_and_moments::CoordinateSystem;

use crate::elliptic_wing_theory::EllipticalWing;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmpiricalAngleOfAttackCorrection {
    pub factor: Float,
    pub nr_solver_iterations: usize,
    pub solver_damping_factor: Float,
}

impl EmpiricalAngleOfAttackCorrection {
    pub fn angles_of_attack_correction(
        &self,
        line_force_model: &LineForceModel,
        ctrl_points_velocity: &[SpatialVector],
    ) -> Vec<Float> {
        let section_cl = line_force_model.lift_coefficients(
            ctrl_points_velocity, 
            CoordinateSystem::Global
        );

        let wing_cl = line_force_model.wing_averaged_values(&section_cl);

        let aspect_ratios = line_force_model.aspect_ratios();

        let nr_span_lines = line_force_model.nr_span_lines();

        let mut angles = vec![0.0; nr_span_lines];

        for i in 0..nr_span_lines {
            let wing_index = line_force_model.wing_index_from_global(i);

            let theory = EllipticalWing {
                aspect_ratio: aspect_ratios[wing_index],
            };

            angles[i] = theory.lift_induced_angle_of_attach(wing_cl[wing_index]) * self.factor;
        }

        angles

    }

    pub fn solve_correction(
        &self, 
        line_force_model: &LineForceModel,
        ctrl_points_velocity: &[SpatialVector]
    ) -> Vec<SpatialVector> {

        let nr_span_lines = line_force_model.nr_span_lines();
        let span_lines = line_force_model.span_lines();

        let mut corrected_ctrl_points_velocity = ctrl_points_velocity.to_vec();

        let mut angle_correction = vec![0.0; nr_span_lines];

        for _ in 0..self.nr_solver_iterations {
            let new_estimated_angles = self.angles_of_attack_correction(
                line_force_model, 
                &corrected_ctrl_points_velocity
            );

            for j in 0..nr_span_lines {
                angle_correction[j] += self.solver_damping_factor * (
                    new_estimated_angles[j] - angle_correction[j]
                );
            }

            for j in 0..nr_span_lines {
                let axis = span_lines[j].relative_vector().normalize();
                
                corrected_ctrl_points_velocity[j] = ctrl_points_velocity[j].rotate_around_axis(
                    -angle_correction[j], 
                    axis
                );
            }
        }

        corrected_ctrl_points_velocity
    }
}