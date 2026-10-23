// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Moving average filter for the measured flow conditions used as input to the controller.

use stormath::smoothing::moving_average::MovingAverage;
use stormath::type_aliases::Float;

use super::input::ControllerInput;

#[derive(Debug, Clone)]
/// Moving average filter for the measured values in the input to the controller for a single
/// wing, so that the controller can act on a smoothed version of the flow conditions, rather than
/// the instantaneous values.
///
/// Only the measured values, i.e., the angle of attack, the velocity and the apparent wind
/// direction, are filtered. The apparent wind direction is averaged through its sine and cosine,
/// so that the average is correct also when the direction passes +/- pi.
pub struct ControllerInputFilter {
    angle_of_attack: MovingAverage,
    velocity: MovingAverage,
    apparent_wind_direction_sin: MovingAverage,
    apparent_wind_direction_cos: MovingAverage,
}

impl ControllerInputFilter {
    /// Creates a new filter that averages over the given number of inputs
    pub fn new(window_size: usize) -> Self {
        Self {
            angle_of_attack: MovingAverage::new(window_size),
            velocity: MovingAverage::new(window_size),
            apparent_wind_direction_sin: MovingAverage::new(window_size),
            apparent_wind_direction_cos: MovingAverage::new(window_size),
        }
    }

    /// Adds a new input to the filter, and returns a copy of the input where the measured values
    /// are replaced by their moving averages
    pub fn filter(&mut self, input: &ControllerInput) -> ControllerInput {
        self.angle_of_attack.add(input.angle_of_attack);
        self.velocity.add(input.velocity);
        self.apparent_wind_direction_sin.add(input.apparent_wind_direction.sin());
        self.apparent_wind_direction_cos.add(input.apparent_wind_direction.cos());

        let apparent_wind_direction: Float = self.apparent_wind_direction_sin.get_average()
            .atan2(self.apparent_wind_direction_cos.get_average());

        ControllerInput {
            angle_of_attack: self.angle_of_attack.get_average(),
            velocity: self.velocity.get_average(),
            apparent_wind_direction,
            ..input.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use stormath::consts::PI;

    use crate::test_utils::tolerance;

    fn input(angle_of_attack: Float, velocity: Float, apparent_wind_direction: Float) -> ControllerInput {
        ControllerInput {
            loading: 0.7,
            current_local_wing_angle: 0.1,
            current_internal_state: 2.0,
            angle_of_attack,
            velocity,
            apparent_wind_direction,
        }
    }

    #[test]
    fn filter_averages_the_last_values() {
        let mut filter = ControllerInputFilter::new(3);

        filter.filter(&input(0.1, 4.0, 0.5));
        filter.filter(&input(0.2, 5.0, 0.5));
        filter.filter(&input(0.3, 6.0, 0.5));

        // The first value is now outside the window
        let filtered = filter.filter(&input(0.4, 7.0, 0.5));

        let tol = tolerance(1e-12, 1e-6);

        assert!((filtered.angle_of_attack - 0.3).abs() < tol);
        assert!((filtered.velocity - 6.0).abs() < tol);
        assert!((filtered.apparent_wind_direction - 0.5).abs() < tol);

        // Values that are not measurements are passed through unchanged
        assert_eq!(filtered.loading, 0.7);
        assert_eq!(filtered.current_local_wing_angle, 0.1);
        assert_eq!(filtered.current_internal_state, 2.0);
    }

    #[test]
    fn apparent_wind_direction_is_averaged_across_pi() {
        let mut filter = ControllerInputFilter::new(2);

        filter.filter(&input(0.0, 1.0, PI - 0.1));
        let filtered = filter.filter(&input(0.0, 1.0, -PI + 0.1));

        // The average of the two directions is pi, and not zero as for a plain average
        assert!((filtered.apparent_wind_direction.abs() - PI).abs() < tolerance(1e-12, 1e-6));
    }
}
