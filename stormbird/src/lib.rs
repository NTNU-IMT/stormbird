// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

#![doc(html_no_source)]

#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]

pub mod common_utils;
pub mod section_models;
pub mod line_force_model;
pub mod lifting_line;
pub mod actuator_line;
pub mod controllers;
pub mod wind;
pub mod error;
pub mod io_utils;
pub mod elliptic_wing_theory;
