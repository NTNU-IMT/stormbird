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

    let cl_theory = TAU * angle_of_attack.sin();
    let cl_model = foil.lift_coefficient(angle_of_attack);

    let cl_error = (cl_model - cl_theory).abs();

    dbg!(&cl_error);

    assert!(cl_error < 1e-5);
}

#[test]
fn minimum_drag_at_zero_lift() {
    let foil = Foil {
        cl_zero_angle: 0.8,
        cd_min: 0.01,
        cd_second_order_factor: 1.5,
        ..Default::default()
    };

    let zero_lift_angle = (-foil.cl_zero_angle / foil.cl_initial_slope).asin();

    let cl_linear = foil.lift_coefficient_linear(zero_lift_angle);
    let cd_at_zero_lift = foil.drag_coefficient_pre_stall(zero_lift_angle);

    dbg!(zero_lift_angle.to_degrees(), cl_linear, cd_at_zero_lift);

    assert!(cl_linear.abs() < 1e-12);
    assert!((cd_at_zero_lift - foil.cd_min).abs() < 1e-12);

    let delta_angle = Float::from(3.0).to_radians();

    let cd_above = foil.drag_coefficient_pre_stall(zero_lift_angle + delta_angle);
    let cd_below = foil.drag_coefficient_pre_stall(zero_lift_angle - delta_angle);

    assert!(cd_above > foil.cd_min);
    assert!(cd_below > foil.cd_min);

    // Without camber, the second order term should be the same as before the change, i.e.,
    // approximately proportional to the angle of attack squared.
    let symmetric_foil = Foil {
        cd_second_order_factor: 1.5,
        ..Default::default()
    };

    let cd_symmetric = symmetric_foil.drag_coefficient_pre_stall(delta_angle);
    let cd_old_model = 1.5 * delta_angle.powi(2);

    assert!((cd_symmetric - cd_old_model).abs() / cd_old_model < 1e-2);
}

/// Derivative of the pre-stall lift with respect to sin(angle_of_attack), computed with central
/// differences.
fn pre_stall_slope(foil: &Foil, angle_of_attack: Float) -> Float {
    let delta = 1e-6;

    let cl_plus  = foil.lift_coefficient_pre_stall_raw(angle_of_attack + delta);
    let cl_minus = foil.lift_coefficient_pre_stall_raw(angle_of_attack - delta);

    let x_plus  = (angle_of_attack + delta).sin();
    let x_minus = (angle_of_attack - delta).sin();

    (cl_plus - cl_minus) / (x_plus - x_minus)
}

#[test]
fn lift_slope_reduction() {
    let reduction_factor = 0.4;

    let foil = Foil {
        cl_zero_angle: 0.5,
        cl_slope_reduction_factor_positive: reduction_factor,
        cl_slope_reduction_onset_angle_positive: Float::from(8.0).to_radians(),
        cl_slope_reduction_range: Float::from(4.0).to_radians(),
        ..Default::default()
    };

    // The lift at zero angle of attack is not affected by the reduction
    let cl_zero = foil.lift_coefficient_pre_stall_raw(0.0);

    assert!((cl_zero - foil.cl_zero_angle).abs() < 1e-12);

    // Below the onset angle, and for negative angles, the slope is the initial slope
    let slope_below = pre_stall_slope(&foil, Float::from(2.0).to_radians());
    let slope_negative = pre_stall_slope(&foil, Float::from(-10.0).to_radians());

    // Above the onset angle, the slope is reduced by the reduction factor
    let slope_above = pre_stall_slope(&foil, Float::from(16.0).to_radians());

    let slope_above_expected = foil.cl_initial_slope * (1.0 - reduction_factor);

    dbg!(slope_below, slope_negative, slope_above, slope_above_expected);

    assert!((slope_below - foil.cl_initial_slope).abs() / foil.cl_initial_slope < 1e-2);
    assert!((slope_negative - foil.cl_initial_slope).abs() / foil.cl_initial_slope < 1e-6);
    assert!((slope_above - slope_above_expected).abs() / slope_above_expected < 1e-2);
}

#[test]
fn lift_slope_reduction_negative_side() {
    let foil = Foil {
        cl_zero_angle: -0.5,
        cl_slope_reduction_factor_negative: 1.0,
        cl_slope_reduction_onset_angle_negative: Float::from(8.0).to_radians(),
        cl_slope_reduction_range: Float::from(4.0).to_radians(),
        ..Default::default()
    };

    // A reduction factor of 1.0 should give zero slope well past the onset angle
    let slope_above = pre_stall_slope(&foil, Float::from(-16.0).to_radians());
    let slope_positive = pre_stall_slope(&foil, Float::from(10.0).to_radians());

    dbg!(slope_above, slope_positive);

    assert!(slope_above.abs() / foil.cl_initial_slope < 1e-2);
    assert!((slope_positive - foil.cl_initial_slope).abs() / foil.cl_initial_slope < 1e-6);
}

#[test]
fn lift_slope_reduction_is_smooth_at_zero_angle() {
    // Different settings for the positive and negative side should not give a jump in the slope
    // at zero angle of attack
    let foil = Foil {
        cl_slope_reduction_factor_positive: 0.8,
        cl_slope_reduction_factor_negative: 0.2,
        cl_slope_reduction_onset_angle_positive: Float::from(3.0).to_radians(),
        cl_slope_reduction_onset_angle_negative: Float::from(6.0).to_radians(),
        ..Default::default()
    };

    let delta = Float::from(1e-4);

    let slope_left  = pre_stall_slope(&foil, -delta);
    let slope_right = pre_stall_slope(&foil, delta);

    dbg!(slope_left, slope_right);

    assert!((slope_left - slope_right).abs() / foil.cl_initial_slope < 1e-3);
}

#[test]
fn lift_slope_reduction_with_zero_range() {
    let foil = Foil {
        cl_slope_reduction_factor_positive: 0.5,
        cl_slope_reduction_range: 0.0,
        ..Default::default()
    };

    let angle = Float::from(15.0).to_radians();

    let x = angle.sin();
    let x_onset = foil.cl_slope_reduction_onset_angle_positive.sin();

    let cl_expected = foil.cl_initial_slope * (x - 0.5 * (x - x_onset));
    let cl_model = foil.lift_coefficient_pre_stall_raw(angle);

    assert!((cl_model - cl_expected).abs() < 1e-12);
    assert!(cl_model.is_finite());
}
