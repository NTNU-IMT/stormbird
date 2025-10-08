// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see seperate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use crate::section_models::foil::Foil;

use stormath::type_aliases::Float;
use stormath::consts::TAU;

#[test]
fn large_angle_of_attack() {
    let foil = Foil::default();

    let angle_of_attack = Float::from(80.0).to_radians();

    let cd = foil.drag_coefficient(angle_of_attack);

    dbg!(cd);

}

#[test]
fn default_lift_coefficent() {
    let foil = Foil::default();

    let angle_of_attack = Float::from(5.0).to_radians();

    let cl_theory = TAU * angle_of_attack;
    let cl_model = foil.lift_coefficient(angle_of_attack);

    let cl_error = (cl_model - cl_theory).abs();

    dbg!(&cl_error);

    assert!(cl_error < 1e-5);
}