// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Various sectional forces from simulations.

use crate::line_force_model::LineForceModel;

use stormath::spatial_vector::SpatialVector;

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
pub enum CoordinateSystem {
    #[default]
    Global,
    Body,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
/// Integrated values representing either forces or moments.
pub struct IntegratedValues {
    pub circulatory: SpatialVector<3>,
    pub sectional_drag: SpatialVector<3>,
    pub added_mass: SpatialVector<3>,
    pub gyroscopic: SpatialVector<3>,
    pub total: SpatialVector<3>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
/// Input data to calculate sectional forces.
pub struct SectionalForcesInput {
    /// Circulation strength of each line element
    pub circulation_strength: Vec<f64>,
    /// The *felt* velocity at each control point, meaning the velocity of the fluid from the 
    /// perspective of the wings, **not** the velocity of the wings themselves.  
    pub velocity: Vec<SpatialVector<3>>,
    /// The estimated angle of attack at each control point.
    pub angles_of_attack: Vec<f64>,
    /// The *felt* acceleration at each control point, meaning the acceleration of the fluid from 
    /// the perspective of the wings, **not** the acceleration of the wings themselves.
    pub acceleration: Vec<SpatialVector<3>>,
    /// The rotational velocity of the entire system. Primarily relevant for gyroscopic effects.
    pub rotation_velocity: SpatialVector<3>,
    /// The coordinate system of the input data.
    pub coordinate_system: CoordinateSystem,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
/// Structures used to store sectional forces from simulations.
pub struct SectionalForces {
    /// Forces due to the circulation on a line element. Computed from the lift part of the 
    /// sectional model.
    pub circulatory: Vec<SpatialVector<3>>,
    /// Forces due to the two dimensional drag on a line element. 
    /// 
    /// **Note**: this is often the viscous drag, but not always. In can also include three- 
    /// dimensional effects on the drag, if the model is executed with a simplified approach, for 
    /// instance when neglecting the *self-induced* velocities.
    pub sectional_drag: Vec<SpatialVector<3>>,
    /// Added mass forces on the line element.
    pub added_mass: Vec<SpatialVector<3>>,
    /// Forces due to gyroscopic effects on the line element.
    pub gyroscopic: Vec<SpatialVector<3>>,
    /// Total forces
    pub total: Vec<SpatialVector<3>>,
    /// The coordinate system of the forces data.
    pub coordinate_system: CoordinateSystem,
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

    /// Calculates the moment contribution from each line element.
    /// 
    /// The moments are calculated as the cross product of the control point and the sectional force.
    pub fn sectional_moments(line_force_model: &LineForceModel, sectional_forces: &[SpatialVector<3>], coordinate_system: CoordinateSystem) -> Vec<SpatialVector<3>> {
        let span_lines = match coordinate_system {
            CoordinateSystem::Global => line_force_model.span_lines(),
            CoordinateSystem::Body => line_force_model.span_lines_local.clone(),
        };

        (0..span_lines.len()).map(
            |index| {
                span_lines[index].ctrl_point().cross(sectional_forces[index])
            }
        ).collect()
    }

    pub fn integrate_forces(&self, line_force_model: &LineForceModel) -> Vec<IntegratedValues> {
        let mut integrated_values: Vec<IntegratedValues> = Vec::new();

        for wing_indices in &line_force_model.wing_indices {
            let mut wing_result = IntegratedValues::default();

            for i in wing_indices.start..wing_indices.end {
                wing_result.circulatory += self.circulatory[i];
                wing_result.sectional_drag += self.sectional_drag[i];
                wing_result.added_mass += self.added_mass[i];
                wing_result.gyroscopic += self.gyroscopic[i];
                wing_result.total += self.total[i];
            }

            integrated_values.push(wing_result);
        }

        integrated_values
    }

    pub fn integrate_moments(&self, line_force_model: &LineForceModel) -> Vec<IntegratedValues> {
        let sectional_circulatory_moments = Self::sectional_moments(line_force_model, &self.circulatory, self.coordinate_system);
        let sectional_drag_moments = Self::sectional_moments(line_force_model, &self.sectional_drag, self.coordinate_system);
        let sectional_added_mass_moments = Self::sectional_moments(line_force_model, &self.added_mass, self.coordinate_system);
        let sectional_gyroscopic_moments = Self::sectional_moments(line_force_model, &self.gyroscopic, self.coordinate_system);
        let sectional_total_moments = Self::sectional_moments(line_force_model, &self.total, self.coordinate_system);

        let mut integrated_values: Vec<IntegratedValues> = Vec::new();

        for wing_indices in &line_force_model.wing_indices {
            let mut wing_result = IntegratedValues::default();

            for i in wing_indices.start..wing_indices.end {
                wing_result.circulatory += sectional_circulatory_moments[i];
                wing_result.sectional_drag += sectional_drag_moments[i];
                wing_result.added_mass += sectional_added_mass_moments[i];
                wing_result.gyroscopic += sectional_gyroscopic_moments[i];
                wing_result.total += sectional_total_moments[i];
            }

            integrated_values.push(wing_result);
        }
        
        integrated_values
    }
}