// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Implementations of wake models used to calculate induced velocities in lifting line simulations

pub mod steady;
pub mod unsteady;
pub mod velocity_corrections;

/// Typical imports when using the velocity models
pub mod prelude {
    pub use super::steady::{SteadyWakeBuilder, SteadyWake};
    pub use super::unsteady::{UnsteadyWakeBuilder, UnsteadyWake};
}

#[cfg(test)]
mod tests;