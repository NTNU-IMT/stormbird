// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Implementations of wake models used to calculate induced velocities in lifting line simulations

pub mod dynamic_wake;
pub mod prelude;
pub mod settings;
pub mod frozen_wake;

use dynamic_wake::DynamicWake;
use settings::QuasiSteadyWakeSettings;

#[derive(Clone, Debug)]
pub enum WakeData {
    Dynamic(DynamicWake),
    QuasiSteady(QuasiSteadyWakeSettings),
}






