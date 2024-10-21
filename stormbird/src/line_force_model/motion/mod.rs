// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)


pub mod rigid_body_motion;
pub mod derivatives;

/// Enum to decide how the motion derivative are determined in a simulation.
/// 
/// There are three options:
/// - AllCalculated: All motion derivatives are calculated numerically, using the finite difference 
/// scheme, based on the current and past state of the line force model.
/// - VelocitySetAccelerationCalculated: the felt velocity due to motion of the control points are 
/// set, but the acceleration is calculated based on the current and past input.
/// - AllSet: The felt velocity and acceleration due to motion of the control points are set, and 
/// no further calculations are done.
pub enum MotionDerivatives {
    AllCalculated,
    VelocitySetAccelerationCalculated,
    AllSet,
}