// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! The velocity input module contains structures and functions to represent the **input** velocity
//! to different simulations. That is, all velocity components that are not calculated by the
//! simulation methods themselves. 
//! 
//! In short, the functionality covers two main things: 1) various ways of representing the 
//! freestream velocity, and 2) methods to calculate all relevant motion parameters due to moving 
//! wings during a simulation. These two things are kept separate as both are not always needed. For
//! instance, the freestream functionality is only relevant for lifting line simulations, as 
//! actuator line simulations uses external CFD solvers to set up the freestream environment. 
//! However, the motion functionality is relevant for both lifting line and actuator line 
//! simulations.  

use crate::vec3::Vec3;

pub mod freestream;
pub mod motion;

use freestream::Freestream;

#[derive(Debug, Clone, Copy)]
/// Structure to store input to a simulation
pub struct InputState {
    /// Freestream velocity, measured in m/s
    pub freestream: Freestream,
    /// Translation of the wing(s), measured in m
    pub translation: Vec3,
    /// Rotation of the wing(s), measured in rad
    pub rotation: Vec3,
}