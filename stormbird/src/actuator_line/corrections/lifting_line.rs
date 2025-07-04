

use crate::lifting_line::wake::{
    Wake,
    frozen_wake::FrozenWake,
    builders::SteadyWakeBuilder,
    builders::ViscousCoreLength,
    line_force_model_data::LineForceModelData,
};

use crate::lifting_line::singularity_elements::symmetry_condition::SymmetryCondition;

use crate::line_force_model::LineForceModel;

use crate::common_utils::prelude::*;

use stormath::spatial_vector::SpatialVector;

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LiftingLineCorrectionBuilder {
    #[serde(default = "LiftingLineCorrectionBuilder::default_wake_length_factor")]
    pub wake_length_factor: f64,
    #[serde(default = "LiftingLineCorrectionBuilder::default_solver_damping_factor")]
    pub solver_damping_factor: f64,
    #[serde(default = "LiftingLineCorrectionBuilder::default_nr_solver_iterations")]
    pub nr_solver_iterations: usize,
}

impl LiftingLineCorrectionBuilder {
    fn default_wake_length_factor() -> f64 {100.0}
    fn default_solver_damping_factor() -> f64 {0.1}
    fn default_nr_solver_iterations() -> usize {20}

    pub fn build(
        &self, 
        viscous_core_length: f64, 
        line_force_model: &LineForceModel
    ) -> LiftingLineCorrection {
        let chord_lengths: Vec<f64> = line_force_model.chord_vectors_local.iter().map(
            |chord_vector| chord_vector.length()
        ).collect();

        let average_chord_length = chord_lengths.iter().sum::<f64>() / chord_lengths.len() as f64;

        let wake_viscous_core = SteadyWakeBuilder {
            wake_length_factor: 100.0,
            symmetry_condition: SymmetryCondition::NoSymmetry,
            viscous_core_length: ViscousCoreLength::Absolute(viscous_core_length * average_chord_length),
        }.build(line_force_model);

        let wake_default = SteadyWakeBuilder {
            wake_length_factor: 100.0,
            symmetry_condition: SymmetryCondition::NoSymmetry,
            viscous_core_length: ViscousCoreLength::default(),
        }.build(line_force_model);

        let nr_of_lines = line_force_model.nr_span_lines();

        let frozen_wake_viscous_core = FrozenWake::initialize(nr_of_lines);
        let frozen_wake_default = FrozenWake::initialize(nr_of_lines);

        let felt_ctrl_point_freestream = vec![SpatialVector([0.0, 0.0, 0.0]); nr_of_lines];

        let previous_line_force_model_data = LineForceModelData::new(
            &line_force_model,
            &felt_ctrl_point_freestream,
            &felt_ctrl_point_freestream,
        );

        let velocity_correction_estimate = vec![SpatialVector::default(); nr_of_lines];

        LiftingLineCorrection {
            wake_viscous_core,
            wake_default,
            frozen_wake_viscous_core,
            frozen_wake_default,
            previous_line_force_model_data,
            velocity_correction_estimate,
            initialized: false,
            nr_solver_iterations: self.nr_solver_iterations,
            solver_damping_factor: self.solver_damping_factor,
        }
    }
}

#[derive(Debug, Clone)]
/// A structure used to compute corrections for the velocity, based on a lifting line model.
pub struct LiftingLineCorrection {
    wake_viscous_core: Wake,
    wake_default: Wake,
    frozen_wake_viscous_core: FrozenWake,
    frozen_wake_default: FrozenWake,
    previous_line_force_model_data: LineForceModelData,
    velocity_correction_estimate: Vec<SpatialVector<3>>,
    pub initialized: bool,
    pub nr_solver_iterations: usize,
    pub solver_damping_factor: f64,
}

impl LiftingLineCorrection {
    pub fn initialize_line_force_model_data(
        &mut self,
        line_force_model: &LineForceModel,
        ctrl_points_velocity: &[SpatialVector<3>],
    ) {
        self.previous_line_force_model_data = LineForceModelData::new(
            line_force_model,
            ctrl_points_velocity,
            ctrl_points_velocity,
        );
    }

    pub fn initialize_with_velocity_and_time_step(
        &mut self,
        line_force_model: &LineForceModel,
        wake_building_velocity: SpatialVector<3>,
        time_step: f64,
    ) {
        self.wake_viscous_core.initialize_with_velocity_and_time_step(
            line_force_model,
            wake_building_velocity,
            time_step,
        );

        self.wake_default.initialize_with_velocity_and_time_step(
            line_force_model,
            wake_building_velocity,
            time_step,
        );

        self.frozen_wake_viscous_core.update(&self.wake_viscous_core);
        self.frozen_wake_default.update(&self.wake_default);

        self.initialized = true;
    }

    pub fn correction_for_lift_induced_velocities(&self, circulation_strength: &[f64]) -> Vec<SpatialVector<3>> {
        let u_i_viscous_core = self.frozen_wake_viscous_core
            .induced_velocities_at_control_points(circulation_strength);

        let u_i_default = self.frozen_wake_default
            .induced_velocities_at_control_points(circulation_strength);

        let mut u_i_correction = Vec::with_capacity(u_i_viscous_core.len());

        for i in 0..u_i_viscous_core.len() {
            u_i_correction.push(u_i_default[i] - u_i_viscous_core[i]);
        }

        u_i_correction
    }

    pub fn update_before_solving(
        &mut self, 
        time_step: f64,
        line_force_model: &LineForceModel
    ) {
        self.wake_viscous_core.update_before_solving(
            time_step, 
            line_force_model, 
            &self.previous_line_force_model_data
        );

        self.wake_default.update_before_solving(
            time_step, 
            line_force_model, 
            &self.previous_line_force_model_data
        );

        self.frozen_wake_viscous_core.update(
            &self.wake_viscous_core
        );

        self.frozen_wake_default.update(
            &self.wake_default
        );
    }

    pub fn wake_point_freestream_from_ctrl_point_values(
        &self,
        ctrl_points_velocity: &[SpatialVector<3>]
    ) -> Vec<SpatialVector<3>> {
        let averaged_velocity = ctrl_points_velocity.iter().sum::<SpatialVector<3>>() / ctrl_points_velocity.len() as f64;

        let nr_wake_points = self.wake_viscous_core.points.len();

        let wake_points_freestream = vec![averaged_velocity; nr_wake_points];

        wake_points_freestream
    }

    pub fn update_after_solving(
        &mut self, 
        line_force_model: &LineForceModel,
        new_circulation_strength: &[f64],
        ctrl_points_velocity: &[SpatialVector<3>]
    ) {
        let wake_points_freestream = self.wake_point_freestream_from_ctrl_point_values(
            ctrl_points_velocity
        );

        self.wake_viscous_core.update_after_solving(
            new_circulation_strength, 
            &wake_points_freestream
        );

        self.wake_default.update_after_solving(
            new_circulation_strength, 
            &wake_points_freestream
        );

        self.previous_line_force_model_data = LineForceModelData::new(
            line_force_model,
            ctrl_points_velocity,
            ctrl_points_velocity,
        );
    }

    pub fn velocity_correction(
        &mut self,
        time_step: f64,
        line_force_model: &LineForceModel,
        circulation_strength: &[f64],
        ctrl_points_velocity: &[SpatialVector<3>],
    ) -> Vec<SpatialVector<3>> {
        if !self.initialized {
            let mut wake_building_velocity = SpatialVector::<3>::default();

            for i in 0..ctrl_points_velocity.len() {
                wake_building_velocity += ctrl_points_velocity[i];
            }
            wake_building_velocity /= ctrl_points_velocity.len() as f64;
            
            self.initialize_with_velocity_and_time_step(
                &line_force_model, 
                wake_building_velocity, 
                time_step
            );

            self.initialize_line_force_model_data(
                &line_force_model, 
                ctrl_points_velocity
            );
        }

        self.update_before_solving(time_step, &line_force_model);

        let velocity_correction = self
            .correction_for_lift_induced_velocities(circulation_strength);

        self.update_after_solving(
            &line_force_model, 
            circulation_strength,
            ctrl_points_velocity
        );

        velocity_correction

        
    }

    pub fn solve_correction(
        &mut self,
        time_step: f64,
        line_force_model: &LineForceModel,
        ctrl_points_velocity: &[SpatialVector<3>],
        circulation_strength: &[f64],
    ) -> (
        Vec<SpatialVector<3>>,
        Vec<f64>
    ) {
        let mut corrected_ctrl_points_velocity = ctrl_points_velocity.to_vec();
        let mut corrected_circulation_strength = circulation_strength.to_vec();

        for _ in 0..self.nr_solver_iterations {
            let new_velocity_correction_estimate: Vec<SpatialVector<3>> = self.velocity_correction(
                time_step,
                line_force_model,
                &corrected_circulation_strength, 
                &corrected_ctrl_points_velocity
            );

            for j in 0..corrected_ctrl_points_velocity.len() {
                let current_velocity_correction = self.velocity_correction_estimate[j];

                self.velocity_correction_estimate[j] += (
                    new_velocity_correction_estimate[j] - current_velocity_correction
                ) * self.solver_damping_factor;

                corrected_ctrl_points_velocity[j] = ctrl_points_velocity[j] + self.velocity_correction_estimate[j];
            }

            corrected_circulation_strength = line_force_model.circulation_strength(
                &corrected_ctrl_points_velocity, 
                CoordinateSystem::Global
            );
        }

        (corrected_ctrl_points_velocity, corrected_circulation_strength)
    }
}