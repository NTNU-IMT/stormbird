// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html) 

use crate::error::Error;
use crate::line_force_model::LineForceModel;

use serde::{Deserialize, Serialize};

use super::Controller;
use super::set_points::ControllerSetPoints;
use super::measurements::SpanwiseMeasurementBuilder;
use super::input_filter::ControllerInputFilter;

use stormath::type_aliases::Float;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Builder for the controller
pub struct ControllerBuilder {
    /// The set points for each sail
    pub set_points: Vec<ControllerSetPoints>,
    /// How to measure the representative flow conditions on the sails
    #[serde(default)]
    pub spanwise_measurement: SpanwiseMeasurementBuilder,
    /// The number of time steps between each update of the controller
    #[serde(default = "ControllerBuilder::default_time_steps_between_updates")]
    pub time_steps_between_updates: usize,
    /// The time when the controller starts to be used
    #[serde(default)]
    pub start_time: Float,
    /// Optional window size for a moving average of the measured input to the controller, given as
    /// the number of inputs to average over. The filter is only used if the window size is larger
    /// than zero. A new input is added to the average each time the controller is updated, also
    /// when it does not change the output.
    #[serde(default)]
    pub moving_average_window_size: Option<usize>,
    /// If true, the input velocity is used when measuring the apparent wind direction, rather than
    /// the velocity including induced velocities
    #[serde(default)]
    pub use_input_velocity_for_apparent_wind_direction: bool,
}

impl ControllerBuilder {
    /// Default number of time steps between each update (1)
    pub fn default_time_steps_between_updates() -> usize {1}

    /// Creates a builder from a JSON string
    pub fn from_json_string(json_string: &str) -> Result<Self, Error> {
        let serde_res = serde_json::from_str(json_string)?;

        Ok(serde_res)
    }

    /// Creates a builder from a JSON file
    pub fn from_json_file(file_path: &str) -> Result<Self, Error> {
        let json_string = std::fs::read_to_string(file_path)?;
        
        Self::from_json_string(&json_string)
    }

    /// Builds the controller for the given line force model
    pub fn build(&self, line_force_model: &LineForceModel) -> Result<Controller, Error> {
        let input_filters = match self.moving_average_window_size {
            Some(window_size) if window_size > 0 => Some(
                vec![ControllerInputFilter::new(window_size); line_force_model.nr_wings()]
            ),
            _ => None,
        };

        Ok(Controller {
            set_points: self.set_points.clone(),
            spanwise_measurement: self.spanwise_measurement.build(line_force_model)?,
            time_steps_between_updates: self.time_steps_between_updates,
            start_time: self.start_time,
            time_step_index: 0,
            use_input_velocity_for_apparent_wind_direction: self.use_input_velocity_for_apparent_wind_direction,
            input_filters,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::controller::input::ControllerInput;
    use crate::line_force_model::builder::LineForceModelBuilder;
    use crate::line_force_model::builder::single_wing::WingBuilder;
    use crate::section_models::SectionModel;
    use crate::section_models::foil::Foil;
    use crate::test_utils::tolerance;

    use stormath::spatial_vector::SpatialVector;

    fn single_wing_model() -> LineForceModel {
        let chord_vector = SpatialVector::from([5.0, 0.0, 0.0]);

        let mut builder = LineForceModelBuilder::new(5);

        builder.add_wing(WingBuilder {
            section_points: vec![
                SpatialVector::from([0.0, 0.0, 0.0]),
                SpatialVector::from([0.0, 0.0, 30.0]),
            ],
            chord_vectors: vec![chord_vector, chord_vector],
            section_model: SectionModel::SingleElementFoil(Foil::default()),
            ..Default::default()
        });

        builder.build()
    }

    fn controller_builder(moving_average_window_size: Option<usize>) -> ControllerBuilder {
        ControllerBuilder {
            set_points: vec![ControllerSetPoints {
                apparent_wind_directions_data: vec![-1.0, 1.0],
                angle_of_attack_data: Some(vec![0.0, 0.0]),
                use_effective_angle_of_attack: true,
                ..Default::default()
            }],
            spanwise_measurement: SpanwiseMeasurementBuilder::default(),
            time_steps_between_updates: 1,
            start_time: 0.0,
            moving_average_window_size,
            use_input_velocity_for_apparent_wind_direction: false,
        }
    }

    fn input(angle_of_attack: Float) -> ControllerInput {
        ControllerInput {
            angle_of_attack,
            velocity: 10.0,
            ..Default::default()
        }
    }

    #[test]
    fn input_filters_are_only_used_with_a_positive_window_size() {
        let model = single_wing_model();

        assert!(controller_builder(None).build(&model).unwrap().input_filters.is_none());
        assert!(controller_builder(Some(0)).build(&model).unwrap().input_filters.is_none());

        let filters = controller_builder(Some(3)).build(&model).unwrap().input_filters;

        assert_eq!(filters.map(|f| f.len()), Some(model.nr_wings()));
    }

    #[test]
    fn controller_uses_the_filtered_input() {
        let model = single_wing_model();
        let time_step = 0.1;

        // With a zero angle of attack set point, the effective angle of attack controller changes
        // the local wing angle by the measured angle of attack.
        let mut unfiltered = controller_builder(None).build(&model).unwrap();
        let mut filtered = controller_builder(Some(2)).build(&model).unwrap();

        for controller in [&mut unfiltered, &mut filtered] {
            controller.update(0.0, time_step, &[input(0.1)], &model);
        }

        let output_unfiltered = unfiltered.update(time_step, time_step, &[input(0.3)], &model).unwrap();
        let output_filtered = filtered.update(time_step, time_step, &[input(0.3)], &model).unwrap();

        let tol = tolerance(1e-12, 1e-6);

        assert!((output_unfiltered.local_wing_angles[0] - 0.3).abs() < tol);
        assert!((output_filtered.local_wing_angles[0] - 0.2).abs() < tol);
    }
}
