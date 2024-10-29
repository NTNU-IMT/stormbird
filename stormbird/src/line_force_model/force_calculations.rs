// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use super::*;

/// This implementation block contains the functions that calculates the forces on line elements and
/// on the wings. These are generally used as the last step in a simulation method using the
/// line force model.
impl LineForceModel {
    /// Return the angle of attack at each control point.
    ///
    /// The angle is defined as the rotation from the chord vector to the velocity vector, using the
    /// span line as the axis of rotation, with right handed positive rotation.
    ///
    /// # Argument
    /// * `velocity` - the velocity vector at each control point
    pub fn angles_of_attack(&self, velocity: &[SpatialVector<3>], input_coordinate_system: CoordinateSystem) -> Vec<f64> {
        let (span_lines, chord_vectors) = match input_coordinate_system {
            CoordinateSystem::Global => (self.span_lines(), self.global_chord_vectors()),
            CoordinateSystem::Body => (self.span_lines_local.clone(), self.local_chord_vectors()),
        };

        let angles_of_attack: Vec<f64> = (0..velocity.len()).map(|index| {
            chord_vectors[index].signed_angle_between(
                velocity[index],
                span_lines[index].direction()
            )
        }).collect();

        angles_of_attack
    }

    /// Returns the local lift coefficient on each line element.
    ///
    /// # Argument
    /// * `velocity` - the velocity vector at each control point
    pub fn lift_coefficients(&self, velocity: &[SpatialVector<3>], input_coordinate_system: CoordinateSystem) -> Vec<f64> {
        let angles_of_attack = self.angles_of_attack(velocity, input_coordinate_system);

        (0..self.nr_span_lines()).map(
            |index| {
                let wing_index  = self.wing_index_from_global(index);

                match &self.section_models[wing_index] {
                    SectionModel::Foil(foil) =>
                        foil.lift_coefficient(angles_of_attack[index]),
                    SectionModel::VaryingFoil(foil) =>
                        foil.lift_coefficient(angles_of_attack[index]),
                    SectionModel::RotatingCylinder(cylinder) =>
                        cylinder.lift_coefficient(
                            self.chord_vectors_local[index].length(), velocity[index].length()
                        ),
                }
            }
        ).collect()
    }

    /// Returns the circulation strength, either directly or based on the prescribed shape,
    /// depending on the fields in self.
    ///
    /// # Argument
    /// * `velocity` - the velocity vector at each control point
    pub fn circulation_strength(&self, velocity: &[SpatialVector<3>], input_coordinate_system: CoordinateSystem) -> Vec<f64> {
        match &self.circulation_corrections {
            CirculationCorrection::None => self.circulation_strength_raw(velocity, input_coordinate_system),
            CirculationCorrection::PrescribedCirculation(shape) =>
                self.prescribed_circulation_strength(&velocity,&shape, input_coordinate_system),
            CirculationCorrection::GaussianSmoothing(settings) => {
                let raw_strength = self.circulation_strength_raw(velocity, input_coordinate_system);

                self.gaussian_smoothed_values(&raw_strength, &settings)
            },
            CirculationCorrection::PolynomialSmoothing => {
                let raw_strength = self.circulation_strength_raw(velocity, input_coordinate_system);

                self.polynomial_smoothed_values(&raw_strength)
            }
        }
    }

    /// Returns the circulation strength on each line based on the lifting line equation.
    ///
    /// # Argument
    /// * `velocity` - the velocity vector at each control point
    pub fn circulation_strength_raw(&self, velocity: &[SpatialVector<3>], input_coordinate_system: CoordinateSystem) -> Vec<f64> {
        let cl = self.lift_coefficients(&velocity, input_coordinate_system);

        (0..velocity.len()).map(|index| {
            -0.5 * self.chord_vectors_local[index].length() * velocity[index].length() * cl[index]
        }).collect()
    }

    /// Returns the viscous drag coefficient on each line element, based on the section model
    /// and the input velocity.
    ///
    /// # Argument
    /// * `velocity` - the velocity vector at each control point
    pub fn viscous_drag_coefficients(&self, velocity: &[SpatialVector<3>], input_coordinate_system: CoordinateSystem) -> Vec<f64> {
        let angles_of_attack = self.angles_of_attack(velocity, input_coordinate_system);

        (0..self.nr_span_lines()).map(
            |index| {
                let wing_index  = self.wing_index_from_global(index);

                match &self.section_models[wing_index] {
                    SectionModel::Foil(foil) =>
                        foil.drag_coefficient(angles_of_attack[index]),
                    SectionModel::VaryingFoil(foil) =>
                        foil.drag_coefficient(angles_of_attack[index]),
                    SectionModel::RotatingCylinder(cylinder) =>
                        cylinder.drag_coefficient(self.chord_vectors_local[index].length(), velocity[index].length())
                }
            }
        ).collect()
    }

    pub fn sectional_force_input(&self, solver_result: &SolverResult, time_step: f64) -> SectionalForcesInput {
        let angles_of_attack = self.angles_of_attack(&solver_result.ctrl_point_velocity, CoordinateSystem::Global);

        let nr_span_lines = self.nr_span_lines();

        let mut acceleration = vec![SpatialVector::<3>::default(); nr_span_lines];
        let mut angles_of_attack_derivative = vec![0.0; nr_span_lines];
        let mut rotation_velocity = SpatialVector::<3>::default();

        if let Some(derivatives) = &self.derivatives {
            acceleration = derivatives.flow.acceleration(
                &solver_result.ctrl_point_velocity, time_step
            );

            angles_of_attack_derivative = derivatives.flow.angles_of_attack_derivative(
                &angles_of_attack, time_step
            );

            rotation_velocity = derivatives.motion.rotation_velocity(self, time_step);
        }

        let mut velocity = solver_result.ctrl_point_velocity.clone();

        match self.output_coordinate_system {
            CoordinateSystem::Body => {
                for i in 0..nr_span_lines {
                    velocity[i] = velocity[i].in_rotated_coordinate_system(self.rotation);
                    acceleration[i] = acceleration[i].in_rotated_coordinate_system(self.rotation);
                }

                rotation_velocity = rotation_velocity.in_rotated_coordinate_system(self.rotation);
            },
            CoordinateSystem::Global => {}
        }

        SectionalForcesInput {
            circulation_strength: solver_result.circulation_strength.clone(),
            velocity,
            angles_of_attack,
            acceleration,
            angles_of_attack_derivative,
            rotation_velocity,
            coordinate_system: self.output_coordinate_system
        }
    }

     /// Calculates the forces on each line element.
     pub fn sectional_forces(&self, input: &SectionalForcesInput) -> SectionalForces {
        let mut sectional_forces = SectionalForces {
            circulatory: self.sectional_circulatory_forces(&input.circulation_strength, &input.velocity),
            sectional_drag: self.sectional_drag_forces(&input.velocity),
            added_mass: self.sectional_added_mass_force(&input.acceleration),
            gyroscopic: self.sectional_gyroscopic_force(input.rotation_velocity),
            total: vec![SpatialVector::<3>::default(); self.nr_span_lines()],
            coordinate_system: input.coordinate_system
        };

        sectional_forces.compute_total();

        sectional_forces
    }

    /// Calculates the forces on each line element due to the circulatory forces (i.e., sectional lift)
    pub fn sectional_circulatory_forces(&self, strength: &[f64], velocity: &[SpatialVector<3>]) -> Vec<SpatialVector<3>> {
        let span_lines = match self.output_coordinate_system {
            CoordinateSystem::Global => self.span_lines(),
            CoordinateSystem::Body => self.span_lines_local.clone(),
        };

        (0..self.nr_span_lines()).map(
            |index| {
                if velocity[index].length() == 0.0 {
                    SpatialVector::<3>::default()
                } else {
                    strength[index] * velocity[index].cross(span_lines[index].relative_vector()) * self.density
                }
            }
        ).collect()
    }

    /// Calculates the forces on each line element due to the sectional drag model. This is most
    /// often the viscous drag, but it can also include other physical effects if that is included
    /// in the sectional drag model.
    pub fn sectional_drag_forces(&self, velocity: &[SpatialVector<3>]) -> Vec<SpatialVector<3>> {
        let cd = self.viscous_drag_coefficients(velocity, self.output_coordinate_system);

        (0..self.nr_span_lines()).map(
            |index| {
                let drag_direction = velocity[index].normalize();

                let drag_area = self.chord_vectors_local[index].length() * self.span_lines_local[index].length();

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
    pub fn sectional_added_mass_force(&self, acceleration: &[SpatialVector<3>]) -> Vec<SpatialVector<3>> {
        let (span_lines, chord_vectors) = match self.output_coordinate_system {
            CoordinateSystem::Global => (self.span_lines(), self.global_chord_vectors()),
            CoordinateSystem::Body => (self.span_lines_local.clone(), self.local_chord_vectors())
        };

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
    pub fn sectional_gyroscopic_force(&self, rotation_velocity: SpatialVector<3>) -> Vec<SpatialVector<3>> {
        let span_lines = match self.output_coordinate_system {
            CoordinateSystem::Global => self.span_lines(),
            CoordinateSystem::Body => self.span_lines_local.clone(),
        };

        (0..self.nr_span_lines()).map(
            |index| {
                let wing_index = self.wing_index_from_global(index);

                match &self.section_models[wing_index] {
                    SectionModel::Foil(_) | SectionModel::VaryingFoil(_) => SpatialVector::<3>::default(),
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

    /// Calculates the magnitude of the lift force on each line element based on the given
    /// circulation and velocity.
    pub fn lift_from_circulation(&self, strength: &[f64], velocity: &[SpatialVector<3>]) -> Vec<f64> {
        let force = self.sectional_circulatory_forces(strength, velocity);

        force.iter().map(|f| f.length()).collect()
    }

    /// Calculates the magnitude of the lift force on each line element based on the given
    /// coefficients and velocity
    pub fn lift_from_coefficients(&self, velocity: &[SpatialVector<3>], input_coordinate_system: CoordinateSystem) -> Vec<f64> {
        let cl = self.lift_coefficients(velocity, input_coordinate_system);

        (0..self.nr_span_lines()).map(
            |i| {
                let chord = self.chord_vectors_local[i].length();
                let lift_area = chord * self.span_lines_local[i].length();

                let force_factor = 0.5 * lift_area * self.density * velocity[i].length().powi(2);

                cl[i] * force_factor
            }
        ).collect()
    }

    pub fn residual(&self, strength: &[f64], velocity: &[SpatialVector<3>], input_coordinate_system: CoordinateSystem) -> Vec<f64> {
        let circulation_lift = self.lift_from_circulation(strength, velocity);
        let lift_coefficients = self.lift_coefficients(velocity, input_coordinate_system);

        (0..self.nr_span_lines()).map(|i_span| {
            let chord = self.chord_vectors_local[i_span].length();
                let lift_area = chord * self.span_lines_local[i_span].length();

                let force_factor = 0.5 * lift_area * self.density * velocity[i_span].length().powi(2);

                circulation_lift[i_span] / force_factor - lift_coefficients[i_span]
        }).collect()
    }

    pub fn residual_absolute(&self, strength: &[f64], velocity: &[SpatialVector<3>], input_coordinate_system: CoordinateSystem) -> Vec<f64> {
        self.residual(strength, velocity, input_coordinate_system).iter().map(|r| r.abs()).collect()
    }

    pub fn average_residual_absolute(&self, strength: &[f64], velocity: &[SpatialVector<3>], input_coordinate_system: CoordinateSystem) -> f64 {
        let residuals = self.residual_absolute(strength, velocity, input_coordinate_system);

        residuals.iter().sum::<f64>() / residuals.len() as f64
    }

    /// Function that calculates the amount of flow separation, as predicted by the sectional models
    /// based on the angles of attack on each control point
    pub fn amount_of_flow_separation(&self, angles_of_attack: &[f64]) -> Vec<f64> {
        (0..self.nr_span_lines()).map(
            |i| {
                let wing_index = self.wing_index_from_global(i);

                self.section_models[wing_index].amount_of_flow_separation(angles_of_attack[i])
            }
        ).collect()
    }
}
