// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Settings for the solvers used in the lifting line models.

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone)]
pub struct SolverSettings {
    pub max_iterations_per_time_step: usize,
    pub damping_factor_start: f64,
    pub damping_factor_end: Option<f64>,
    pub convergence_test: ConvergenceTest,
    pub print_log: bool,
    pub only_consider_change_in_angle: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SteadySolverSettings {
    #[serde(default="SteadySolverSettings::default_max_iterations_per_time_step")]
    pub max_iterations_per_time_step: usize,
    #[serde(default="SteadySolverSettings::default_damping_factor")]
    pub damping_factor_start: f64,
    #[serde(default)]
    pub damping_factor_end: Option<f64>,
    #[serde(default)]
    pub convergence_test: ConvergenceTest,
    #[serde(default)]
    pub print_log: bool,
    #[serde(default)]
    pub only_consider_change_in_angle: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnsteadySolverSettings {
    #[serde(default="UnsteadySolverSettings::default_max_iterations_per_time_step")]
    pub max_iterations_per_time_step: usize,
    #[serde(default="UnsteadySolverSettings::default_damping_factor")]
    pub damping_factor_start: f64,
    #[serde(default)]
    pub damping_factor_end: Option<f64>,
    #[serde(default)]
    pub convergence_test: ConvergenceTest,
    #[serde(default)]
    pub print_log: bool,
    #[serde(default)]
    pub only_consider_change_in_angle: bool,
}

impl SteadySolverSettings {
    pub fn default_max_iterations_per_time_step() -> usize {1000}
    pub fn default_damping_factor() -> f64 {0.05}

    pub fn to_solver_settings(&self) -> SolverSettings {
        SolverSettings {
            max_iterations_per_time_step: self.max_iterations_per_time_step,
            damping_factor_start: self.damping_factor_start,
            damping_factor_end: self.damping_factor_end,
            convergence_test: self.convergence_test.clone(),
            print_log: self.print_log,
            only_consider_change_in_angle: self.only_consider_change_in_angle,
        }
    }
}

impl Default for SteadySolverSettings {
    fn default() -> Self {
        SteadySolverSettings {
            max_iterations_per_time_step: Self::default_max_iterations_per_time_step(),
            damping_factor_start: Self::default_damping_factor(),
            damping_factor_end: None,
            convergence_test: Default::default(),
            print_log: Default::default(),
            only_consider_change_in_angle: Default::default(),
        }
    }
}

impl UnsteadySolverSettings {
    pub fn default_max_iterations_per_time_step() -> usize {20}
    pub fn default_damping_factor() -> f64 {0.05}

    pub fn to_solver_settings(&self) -> SolverSettings {
        SolverSettings {
            max_iterations_per_time_step: self.max_iterations_per_time_step,
            damping_factor_start: self.damping_factor_start,
            damping_factor_end: self.damping_factor_end,
            convergence_test: self.convergence_test.clone(),
            print_log: self.print_log,
            only_consider_change_in_angle: self.only_consider_change_in_angle,
        }
    }
}

impl Default for UnsteadySolverSettings {
    fn default() -> Self {
        UnsteadySolverSettings {
            max_iterations_per_time_step: Self::default_max_iterations_per_time_step(),
            damping_factor_start: Self::default_damping_factor(),
            damping_factor_end: None,
            convergence_test: Default::default(),
            print_log: Default::default(),
            only_consider_change_in_angle: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvergenceTest {
    pub minimum_successes: usize,
    pub allowed_error: f64,
}

impl Default for ConvergenceTest {
    fn default() -> Self {
        Self {
            minimum_successes: 5,
            allowed_error: 1e-4,
        }
    }
}

impl ConvergenceTest {
    pub fn build(&self) -> ConvergenceTestExecutor {
        ConvergenceTestExecutor {
            number_of_successes: 0,
            minimum_successes: self.minimum_successes,
            allowed_error: self.allowed_error,
        }
    }
}

pub struct ConvergenceTestExecutor {
    pub number_of_successes: usize,
    pub minimum_successes: usize,
    pub allowed_error: f64,
}

impl ConvergenceTestExecutor {
    pub fn test(&mut self, max_error: f64) -> bool {
        if max_error.abs() < self.allowed_error {
            self.number_of_successes += 1;
        } else {
            self.number_of_successes = 0;
        }

        if max_error.abs() < self.allowed_error && self.number_of_successes >= self.minimum_successes {
            true
        } else {
            false
        }
    }
}