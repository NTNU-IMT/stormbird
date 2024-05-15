// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use super::*;

/// This implementation block contains the functions that calculates the forces on line elements and
/// on the wings. These are generally used as the last step in a simulation method using the 
/// line force model.
impl LineForceModel {
     /// Calculates the forces on each line element.
     pub fn sectional_forces(&self, input: &SectionalForcesInput) -> SectionalForces {
        let mut sectional_forces = SectionalForces {
            circulatory: self.sectional_circulatory_forces(&input.circulation_strength, &input.velocity),
            sectional_drag: self.sectional_drag_forces(&input.velocity),
            added_mass: self.sectional_added_mass_force(&input.acceleration),
            gyroscopic: self.sectional_gyroscopic_force(input.rotation_velocity),
            total: vec![Vec3::default(); self.nr_span_lines()],
        };

        sectional_forces.compute_total();

        sectional_forces
    }

    /// Calculates the forces on each line element due to the circulatory forces (i.e., sectional lift)
    pub fn sectional_circulatory_forces(&self, strength: &[f64], velocity: &[Vec3]) -> Vec<Vec3> {
        let span_lines = self.span_lines();

        (0..self.nr_span_lines()).map(
            |index| {
                if velocity[index].length() == 0.0 {
                    Vec3::default()
                } else {
                    strength[index] * velocity[index].cross(span_lines[index].relative_vector())
                }
            }
        ).collect()
    }

    /// Calculates the forces on each line element due to the sectional drag model. This is most 
    /// often the viscous drag, but it can also include other physical effects if that is included
    /// in the sectional drag model.
    pub fn sectional_drag_forces(&self, velocity: &[Vec3]) -> Vec<Vec3> {
        let span_lines = self.span_lines();
        let cd = self.viscous_drag_coefficients(velocity);

        (0..self.nr_span_lines()).map(
            |index| {
                let drag_direction = velocity[index].normalize();

                let drag_area = self.chord_vectors_local[index].length() * span_lines[index].length();

                let force_factor = 0.5 * drag_area * self.density * velocity[index].length().powi(2);

                drag_direction * cd[index] * force_factor
            }
        ).collect()
    }

    /// Calculates the added mass force on each line element due to the flow acceleration at each 
    /// control point. 
    /// 
    /// **Note**: At the moment, this function only calculates the added mass due to the point 
    /// acceleration. However, according to, for instance, Theodorsen, the added mass should also 
    /// depend on the angular velocity and angular acceleration of the wing. Although these effects
    /// are expected to be small, it should be included in the future. This would, however, require
    /// more information about the motion of the wing to be included as arguments.
    /// 
    /// # Argument
    /// * `acceleration` - the acceleration of the flow at each control point. That is, if the only
    /// velocity is due to the motion of the wings, the acceleration will be opposite to the motion
    /// of the wings.
    pub fn sectional_added_mass_force(&self, acceleration: &[Vec3]) -> Vec<Vec3> {
        let span_lines = self.span_lines();
        let chord_vectors = self.chord_vectors();
        
        (0..self.nr_span_lines()).map(
            |index| {
                let wing_index  = self.wing_index_from_global(index);

                let strip_area = chord_vectors[index].length() * span_lines[index].length();

                let mut relevant_acceleration = acceleration[index];

                relevant_acceleration -= relevant_acceleration.project(span_lines[index].direction());

                match self.section_models[wing_index] {
                    SectionModel::Foil(_) | SectionModel::VaryingFoil(_) => {
                        relevant_acceleration -= relevant_acceleration.project(chord_vectors[index]);
                    },
                    _ => {}
                }

                let added_mass_coefficient = match &self.section_models[wing_index] {
                    SectionModel::Foil(foil) => {
                        foil.added_mass_coefficient(relevant_acceleration.length())
                    },
                    SectionModel::VaryingFoil(foil) => {
                        foil.added_mass_coefficient(relevant_acceleration.length())
                    },
                    SectionModel::RotatingCylinder(cylinder) => {
                        cylinder.added_mass_coefficient(relevant_acceleration.length())
                    }
                };

                added_mass_coefficient * self.density * strip_area * relevant_acceleration.normalize()
            }
        ).collect()
    }

    /// Calculates the gyroscopic force on each line element. This is only relevant for rotor sails.
    /// 
    /// Uses a simplified approach where the rotational speed of the rotor is assumed to be 
    /// significantly larger than the rotational velocity of the sail, for instance due to roll or
    /// pitch motion of the boat.
    pub fn sectional_gyroscopic_force(&self, rotation_velocity: Vec3) -> Vec<Vec3> {
        (0..self.nr_span_lines()).map(
            |index| {
                let wing_index = self.wing_index_from_global(index);
                let span_lines = self.span_lines();

                match &self.section_models[wing_index] {
                    SectionModel::Foil(_) | SectionModel::VaryingFoil(_) => Vec3::default(),
                    SectionModel::RotatingCylinder(cylinder) => {
                        let i_zz = cylinder.moment_of_inertia_2d * span_lines[index].length(); // TODO: does this depend on position?

                        let radial_velocity = 2.0 * PI * cylinder.revolutions_per_second;

                        let angular_momentum = i_zz * radial_velocity * span_lines[index].relative_vector();

                        angular_momentum.cross(rotation_velocity)
                    }
                }

            }
        ).collect()
    }
}