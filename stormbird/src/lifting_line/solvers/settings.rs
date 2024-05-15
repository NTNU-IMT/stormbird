// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Settings for the solvers used in the lifting line models.

use serde::{Serialize, Deserialize};

use crate::line_force_model::solver_utils::ConvergenceTest;

#[derive(Debug, Clone)]
pub struct SolverSettings {
    pub max_iterations_per_time_step: usize,
    pub damping_factor: f64,
    pub circulation_viscosity: f64,
    pub gaussian_smoothing_length: Option<f64>,
    pub convergence_test: ConvergenceTest,
    pub print_log: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SteadySolverSettings {
    #[serde(default="SteadySolverSettings::default_max_iterations_per_time_step")]
    pub max_iterations_per_time_step: usize,
    #[serde(default="SteadySolverSettings::default_damping_factor")]
    pub damping_factor: f64,
    #[serde(default)]
    pub circulation_viscosity: f64,
    #[serde(default)]
    pub gaussian_smoothing_length: Option<f64>,
    #[serde(default)]
    pub convergence_test: ConvergenceTest,
    #[serde(default)]
    pub print_log: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnsteadySolverSettings {
    #[serde(default="UnsteadySolverSettings::default_max_iterations_per_time_step")]
    pub max_iterations_per_time_step: usize,
    #[serde(default="UnsteadySolverSettings::default_damping_factor")]
    pub damping_factor: f64,
    #[serde(default)]
    pub circulation_viscosity: f64,
    #[serde(default)]
    pub gaussian_smoothing_length: Option<f64>,
    #[serde(default)]
    pub convergence_test: ConvergenceTest,
    #[serde(default)]
    pub print_log: bool,
}

impl SteadySolverSettings {
    pub fn default_max_iterations_per_time_step() -> usize {1000}
    pub fn default_damping_factor() -> f64 {0.05}

    pub fn to_solver_settings(&self) -> SolverSettings {
        SolverSettings {
            max_iterations_per_time_step: self.max_iterations_per_time_step,
            damping_factor: self.damping_factor,
            circulation_viscosity: self.circulation_viscosity,
            gaussian_smoothing_length: self.gaussian_smoothing_length,
            convergence_test: self.convergence_test.clone(),
            print_log: self.print_log,
        }
    }
}

impl Default for SteadySolverSettings {
    fn default() -> Self {
        SteadySolverSettings {
            max_iterations_per_time_step: Self::default_max_iterations_per_time_step(),
            damping_factor: Self::default_damping_factor(),
            circulation_viscosity: Default::default(),
            gaussian_smoothing_length: Default::default(),
            convergence_test: Default::default(),
            print_log: Default::default(),
        }
    }
}

impl UnsteadySolverSettings {
    pub fn default_max_iterations_per_time_step() -> usize {20}
    pub fn default_damping_factor() -> f64 {0.1}

    pub fn to_solver_settings(&self) -> SolverSettings {
        SolverSettings {
            max_iterations_per_time_step: self.max_iterations_per_time_step,
            damping_factor: self.damping_factor,
            circulation_viscosity: self.circulation_viscosity,
            gaussian_smoothing_length: self.gaussian_smoothing_length,
            convergence_test: self.convergence_test.clone(),
            print_log: self.print_log,
        }
    }
}

impl Default for UnsteadySolverSettings {
    fn default() -> Self {
        UnsteadySolverSettings {
            max_iterations_per_time_step: Self::default_max_iterations_per_time_step(),
            damping_factor: Self::default_damping_factor(),
            circulation_viscosity: Default::default(),
            gaussian_smoothing_length: Default::default(),
            convergence_test: Default::default(),
            print_log: Default::default(),
        }
    }
}