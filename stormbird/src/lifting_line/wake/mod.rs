// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Implementations of wake models used to calculate induced velocities in lifting line simulations

pub mod dynamic_wake;
pub mod quasi_steady_wake;
pub mod prelude;
pub mod settings;
pub mod frozen_wake;
pub mod vtk;

use stormath::spatial_vector::SpatialVector;

use dynamic_wake::DynamicWake;
use quasi_steady_wake::QuasiSteadyWake;

#[derive(Clone, Debug)]
pub enum WakeData {
    Dynamic(DynamicWake),
    QuasiSteady(QuasiSteadyWake),
}

impl WakeData {
    /// Calculates the induced velocities from the wake at the given points, based on the state of
    /// the wake at the last completed time step.
    pub fn induced_velocities(&self, points: &[SpatialVector]) -> Vec<SpatialVector> {
        match self {
            Self::Dynamic(wake) => wake.induced_velocities(points),
            Self::QuasiSteady(wake) => wake.induced_velocities(points),
        }
    }

    /// Serialize the wake as a legacy VTK PolyData buffer, with the strength and viscous core
    /// length as cell data. See [`vtk::polygons_as_vtk`] for more on the format.
    pub fn wake_as_vtk(&self, binary: bool) -> Vec<u8> {
        match self {
            Self::Dynamic(wake) => wake.wake_as_vtk(binary),
            Self::QuasiSteady(wake) => wake.wake_as_vtk(binary),
        }
    }
}
