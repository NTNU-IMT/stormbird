// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see seperate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use pyo3::prelude::*;

use stormbird::lifting_line::singularity_elements::symmetry_condition::SymmetryCondition as SymmetryConditionRust;

#[pyclass]
#[derive(Clone)]
pub enum SymmetryCondition {
    NoSymmetry,
    X,
    Y,
    Z,
}

impl From<SymmetryCondition> for SymmetryConditionRust {
    fn from(symmetry_condition: SymmetryCondition) -> Self {
        match symmetry_condition {
            SymmetryCondition::NoSymmetry => SymmetryConditionRust::NoSymmetry,
            SymmetryCondition::X => SymmetryConditionRust::X,
            SymmetryCondition::Y => SymmetryConditionRust::Y,
            SymmetryCondition::Z => SymmetryConditionRust::Z,
        }
    }
}