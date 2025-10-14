// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use serde::{Deserialize, Serialize};

use stormath::type_aliases::Float;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// A basic PID controller with anti-windup.
///
/// Source: <https://en.wikipedia.org/wiki/Proportional%E2%80%93integral%E2%80%93derivative_controller>
pub struct PIDController {
    pub proportional_gain: Float,
    pub derivative_gain: Float,
    pub integral_gain: Float,
    pub max_value: Float,
    pub min_value: Float,
    pub reverse_error_sign: bool,
    pub set_point: Float,

    previous_error: Float,
    integral_term: Float,
}

impl PIDController {
    pub fn do_step(&mut self, time_step: Float, input: Float) -> Float {
        let error = if self.reverse_error_sign {
            input - self.set_point
        } else {
            self.set_point - input
        };

        let error_derivative = (error - self.previous_error) / time_step;

        self.integral_term += self.integral_gain * error * time_step;

        if self.integral_term > self.max_value {
            self.integral_term = self.max_value;
        } else if self.integral_term < self.min_value {
            self.integral_term = self.min_value;
        }

        let proportional_term = self.proportional_gain * error;
        let derivative_term   = self.derivative_gain * error_derivative;

        let control_signal = proportional_term + derivative_term + self.integral_term;

        let mut do_anti_windup = false;

        let output = if control_signal > self.max_value {
            do_anti_windup = true;

            self.max_value
        } else if control_signal < self.min_value {
            do_anti_windup = true;

            self.min_value
        } else {
            control_signal
        };

        if do_anti_windup {
            self.integral_term -= control_signal - output;
        }

        self.previous_error = error;

        output
    }
}
