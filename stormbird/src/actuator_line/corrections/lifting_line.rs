

use crate::lifting_line::wake::{
    Wake,
    frozen_wake::FrozenWake,
    builders::SteadyWakeBuilder,
    builders::ViscousCoreLength,
    line_force_model_data::LineForceModelData,
};

use crate::lifting_line::singularity_elements::symmetry_condition::SymmetryCondition;

use crate::line_force_model::LineForceModel;

use stormath::spatial_vector::SpatialVector;

#[derive(Debug, Clone)]
/// A structure used to compute corrections for the velocity, based on a lifting line model.
pub struct LiftingLineCorrection {
    wake_viscous_core: Wake,
    wake_default: Wake,
    frozen_wake_viscous_core: FrozenWake,
    frozen_wake_default: FrozenWake,
    previous_line_force_model_data: LineForceModelData,
    pub initialized: bool,
}

impl LiftingLineCorrection {
    
    pub fn new(viscous_core_length: f64, line_force_model: &LineForceModel) -> Self {
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

        Self {
            wake_viscous_core,
            wake_default,
            frozen_wake_viscous_core,
            frozen_wake_default,
            previous_line_force_model_data,
            initialized: false,
        }
    }

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
        line_force_model: &LineForceModel,
        ctrl_points_velocity: &[SpatialVector<3>]
    ) -> Vec<SpatialVector<3>> {
        let wing_averaged_velocity = line_force_model.wing_averaged_values(
            &ctrl_points_velocity
        );

        let averaged_ctrl_points_velocity = line_force_model.section_values_from_wing_values(
            &wing_averaged_velocity
        );

        let averaged_span_point_values = LineForceModel::span_point_values_from_ctrl_point_values(
            &line_force_model.wing_indices,
            &averaged_ctrl_points_velocity,
            true
        );

        let nr_wake_points = self.wake_viscous_core.points.len();
        let nr_points_per_line_element = self.wake_viscous_core.indices.nr_points_per_line_element;

        let mut wake_points_freestream = Vec::with_capacity(nr_wake_points);

        for _ in 0..nr_points_per_line_element {
            wake_points_freestream.extend_from_slice(&averaged_span_point_values);
        }

        wake_points_freestream
    }

    pub fn update_after_solving(
        &mut self, 
        line_force_model: &LineForceModel,
        new_circulation_strength: &[f64],
        ctrl_points_velocity: &[SpatialVector<3>]
    ) {
        let wake_points_freestream = self.wake_point_freestream_from_ctrl_point_values(
            line_force_model, 
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
}