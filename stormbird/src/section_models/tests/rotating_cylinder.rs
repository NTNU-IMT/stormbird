use stormath::type_aliases::Float;

use crate::section_models::rotating_cylinder::RotatingCylinder;

#[test]
fn rotating_cylinder_power_coefficient_follows_tillig_polynomial() {
    let cylinder = RotatingCylinder::default();

    for spin_ratio in [0.0, 1.0, 2.5, 4.0, -3.0] {
        let sr: Float = Float::abs(spin_ratio);

        let expected = 0.0001 * sr.powi(5) - 0.0004 * sr.powi(4) + 0.0143 * sr.powi(3)
            - 0.0168 * sr.powi(2) + 0.0234 * sr;

        assert!((cylinder.power_coefficient(spin_ratio) - expected).abs() < 1e-12);
    }
}

/// Numerical derivative of the lift coefficient with respect to the spin ratio
fn lift_slope(cylinder: &RotatingCylinder, spin_ratio: Float) -> Float {
    let delta = 1e-5;

    (cylinder.lift_coefficient(spin_ratio + delta) - cylinder.lift_coefficient(spin_ratio - delta)) /
        (2.0 * delta)
}

#[test]
fn rotating_cylinder_lift_is_odd_in_spin_ratio() {
    let cylinder = RotatingCylinder::default();

    assert_eq!(cylinder.lift_coefficient(0.0), 0.0);

    for spin_ratio in [0.1, 1.0, 2.5, 3.4, 4.0, 8.0, 15.0] {
        let cl_positive = cylinder.lift_coefficient(spin_ratio);
        let cl_negative = cylinder.lift_coefficient(-spin_ratio);

        assert!(cl_positive > 0.0);
        assert!((cl_positive + cl_negative).abs() < 1e-12);
    }
}

#[test]
fn rotating_cylinder_lift_slopes_match_parameters() {
    let cylinder = RotatingCylinder {
        cl_initial_slope: 3.0,
        cl_high_spin_ratio_slope: 1.0,
        cl_slope_transition_spin_ratio: 3.0,
        cl_slope_transition_range: 1.0,
        ..Default::default()
    };

    // Well below the transition, the slope is the initial slope, also at zero spin ratio
    for spin_ratio in [0.0, 0.5, -0.5] {
        assert!((lift_slope(&cylinder, spin_ratio) - 3.0).abs() < 1e-4);
    }

    // Half-way through the transition at the transition spin ratio
    for spin_ratio in [3.0, -3.0] {
        assert!((lift_slope(&cylinder, spin_ratio) - 2.0).abs() < 1e-4);
    }

    // Well above the transition, the slope is the high spin ratio slope
    for spin_ratio in [6.0, -6.0, 20.0] {
        assert!((lift_slope(&cylinder, spin_ratio) - 1.0).abs() < 1e-4);
    }
}

#[test]
fn rotating_cylinder_lift_is_linear_with_equal_slopes() {
    let cylinder = RotatingCylinder {
        cl_initial_slope: 2.5,
        cl_high_spin_ratio_slope: 2.5,
        ..Default::default()
    };

    for spin_ratio in [-7.0, -1.0, 0.0, 2.0, 5.0] {
        assert!((cylinder.lift_coefficient(spin_ratio) - 2.5 * spin_ratio).abs() < 1e-12);
    }
}

#[test]
fn rotating_cylinder_lift_has_sharp_transition_with_zero_range() {
    let cylinder = RotatingCylinder {
        cl_initial_slope: 3.0,
        cl_high_spin_ratio_slope: 1.0,
        cl_slope_transition_spin_ratio: 2.0,
        cl_slope_transition_range: 0.0,
        ..Default::default()
    };

    assert!((cylinder.lift_coefficient(1.5) - 4.5).abs() < 1e-12);
    assert!((cylinder.lift_coefficient(4.0) - (6.0 + 2.0)).abs() < 1e-12);
    assert!((cylinder.lift_coefficient(-4.0) + (6.0 + 2.0)).abs() < 1e-12);
}

#[test]
fn rotating_cylinder_default_lift_is_close_to_cfd_data() {
    // The CFD data the default parameters are fitted to
    let spin_ratio_data = [0.0, 0.5, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 8.0];
    let cl_data = [0.0, 1.22, 2.56, 5.93, 9.10, 10.77, 12.80, 13.71, 16.90];

    let cylinder = RotatingCylinder::default();

    for (spin_ratio, cl) in spin_ratio_data.iter().zip(cl_data.iter()) {
        assert!((cylinder.lift_coefficient(*spin_ratio) - cl).abs() < 0.5);
    }
}

#[test]
fn rotating_cylinder_rejects_old_data_based_input() {
    let result = RotatingCylinder::new_from_string(
        r#"{"spin_ratio_data": [0.0, 1.0], "cl_data": [0.0, 2.8]}"#
    );

    assert!(result.is_err());
}
