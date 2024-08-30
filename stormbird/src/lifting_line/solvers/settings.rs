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
    pub tolerance_absolute: f64,
    pub only_consider_change_in_angle: bool,
}

impl SolverSettings {
    pub fn default_tolerance_absolute() -> f64 {1e-4}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SteadySolverSettings {
    #[serde(default="SteadySolverSettings::default_max_iterations_per_time_step")]
    pub max_iterations_per_time_step: usize,
    #[serde(default="SteadySolverSettings::default_damping_factor_start")]
    pub damping_factor_start: f64,
    #[serde(default="SteadySolverSettings::default_damping_factor_end")]
    pub damping_factor_end: Option<f64>,
    #[serde(default="SolverSettings::default_tolerance_absolute")]
    pub tolerance_absolute: f64,
    #[serde(default)]
    pub only_consider_change_in_angle: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnsteadySolverSettings {
    #[serde(default="UnsteadySolverSettings::default_max_iterations_per_time_step")]
    pub max_iterations_per_time_step: usize,
    #[serde(default="UnsteadySolverSettings::default_damping_factor_start")]
    pub damping_factor_start: f64,
    #[serde(default="UnsteadySolverSettings::default_damping_factor_end")]
    pub damping_factor_end: Option<f64>,
    #[serde(default="SolverSettings::default_tolerance_absolute")]
    pub tolerance_absolute: f64,
    #[serde(default)]
    pub only_consider_change_in_angle: bool,
}

impl SteadySolverSettings {
    pub fn default_max_iterations_per_time_step() -> usize {1000}
    pub fn default_damping_factor_start() -> f64 {0.01}
    pub fn default_damping_factor_end() -> Option<f64> {Some(0.05)}

    pub fn to_solver_settings(&self) -> SolverSettings {
        SolverSettings {
            max_iterations_per_time_step: self.max_iterations_per_time_step,
            damping_factor_start: self.damping_factor_start,
            damping_factor_end: self.damping_factor_end,
            tolerance_absolute: self.tolerance_absolute,
            only_consider_change_in_angle: self.only_consider_change_in_angle,
        }
    }
}

impl Default for SteadySolverSettings {
    fn default() -> Self {
        SteadySolverSettings {
            max_iterations_per_time_step: Self::default_max_iterations_per_time_step(),
            damping_factor_start: Self::default_damping_factor_start(),
            damping_factor_end: Self::default_damping_factor_end(),
            tolerance_absolute: SolverSettings::default_tolerance_absolute(),
            only_consider_change_in_angle: Default::default(),
        }
    }
}

impl UnsteadySolverSettings {
    pub fn default_max_iterations_per_time_step() -> usize {1000}
    pub fn default_damping_factor_start() -> f64 {0.01}
    pub fn default_damping_factor_end() -> Option<f64> {Some(0.025)}

    pub fn to_solver_settings(&self) -> SolverSettings {
        SolverSettings {
            max_iterations_per_time_step: self.max_iterations_per_time_step,
            damping_factor_start: self.damping_factor_start,
            damping_factor_end: self.damping_factor_end,
            tolerance_absolute: self.tolerance_absolute,
            only_consider_change_in_angle: self.only_consider_change_in_angle,
        }
    }
}

impl Default for UnsteadySolverSettings {
    fn default() -> Self {
        UnsteadySolverSettings {
            max_iterations_per_time_step: Self::default_max_iterations_per_time_step(),
            damping_factor_start: Self::default_damping_factor_start(),
            damping_factor_end: Self::default_damping_factor_end(),
            tolerance_absolute: SolverSettings::default_tolerance_absolute(),
            only_consider_change_in_angle: Default::default(),
        }
    }
}