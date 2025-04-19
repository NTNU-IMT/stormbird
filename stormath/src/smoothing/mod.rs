// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Smoothing functions for 1D data.

pub mod end_condition;
pub mod gaussian;
pub mod polynomial;
pub mod moving_average;

use end_condition::EndCondition;

use serde::{Serialize, Deserialize};

pub trait SmoothingOps:
    std::ops::Mul<f64, Output = Self> + 
    std::ops::Add<Self, Output = Self> + 
    std::ops::Sub<Self, Output = Self> + 
    std::ops::Div<f64, Output = Self> +
    std::ops::Neg<Output = Self> +
    Default +
    Copy
{}

impl<T> SmoothingOps for T where 
    T:
        std::ops::Mul<f64, Output = T> + 
        std::ops::Add<T, Output = T> + 
        std::ops::Sub<T, Output = T> + 
        std::ops::Div<f64, Output = T> +
        std::ops::Neg<Output = T> +
        Default +
        Copy
{}