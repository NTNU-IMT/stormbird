// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)


//! Models for the wind, including different shapes for the atmospheric boundary layer, gust 
//! spectrums and empirical correction models to account for disturbances from a ship hull.

pub mod inflow_corrections;
pub mod environment;
pub mod wind_condition;
