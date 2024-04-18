// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Various sectional forces from simulations.

use crate::line_force_model::LineForceModel;

use crate::vec3::Vec3;

use serde::{Serialize, Deserialize};

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
/// Integrated values representing either forces or moments.
pub struct IntegratedValues {
    pub circulatory: Vec3,
    pub sectional_drag: Vec3,
    pub added_mass: Vec3,
    pub gyroscopic: Vec3,
    pub total: Vec3,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
/// Structures used to store sectional forces from simulations.
pub struct SectionalForces {
    /// Forces due to the circluation on a line element. Computed from the lift part of the 
    /// sectional model.
    pub circulatory: Vec<Vec3>
    /// Forces due to the two dimensional drag on a line element. 
    /// 
    /// **Note**: this is often the visocus drag, but not always. In can also include three 
    /// dimensional effects on the drag, if the model is executed with a simplfied approach, for 
    /// instance when neglecting the *self-induced* velocities.
    pub sectional_drag: Vec<Vec3>,
    /// Added mass forces on the line element.
    pub added_mass: Vec<Vec3>,
    /// Forces due to gyroscopic effects on the line element.
    pub gyroscopic: Vec<Vec3>,
    /// Total forces
    pub total: Vec<Vec3>,
}

impl SectionalForces {
    pub fn compute_total(&mut self) {
        self.total = self.circulatory.clone();
        
        for i in 0..self.total.len() {
            self.total[i] += self.sectional_drag[i];
            self.total[i] += self.added_mass[i];
            self.total[i] += self.gyroscopic[i];
        }
    }

    pub integrated_forces(&self, line_force_model: &LineForceModel) -> IntegratedValues {
        todo!()
    }

    pub integrated_moments(&self, line_force_model: &LineForceModel) -> IntegratedValues {
        todo!()
    }
}