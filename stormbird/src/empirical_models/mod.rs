// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! A collection of various empirical models that can be used in different simulation methods.
//! 
//! The purpose is to: 
//! 1) Account for effects that are missing from the fundamental theory
//! 2) Modify the results when the prediction from the simplified models are too inaccurate.

/// Models of viscous wakes due to flow separation.
pub mod viscous_wakes;

/// Correction factors that can be applied to a calculated circulation distribution.
pub mod tip_loss_factors;