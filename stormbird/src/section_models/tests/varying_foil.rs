use crate::test_utils::tolerance;
use stormath::type_aliases::Float;
use stormath::interpolation::linear_interpolation;

use crate::section_models::foil::Foil;
use crate::section_models::varying_foil::VaryingFoil;

/// A foil where all parameters that are different for positive and negative angles of attack have
/// different values, so that any mistakes in the mirroring are visible.
fn asymmetric_foil(cl_zero_angle: Float) -> Foil {
    Foil {
        cl_zero_angle,
        cl_initial_slope: 5.5,
        cl_slope_reduction_factor_positive: 0.4,
        cl_slope_reduction_factor_negative: 0.1,
        cl_slope_reduction_onset_angle_positive: Float::to_radians(8.0),
        cl_slope_reduction_onset_angle_negative: Float::to_radians(14.0),
        cl_slope_reduction_range: Float::to_radians(4.0),
        cl_max_after_stall: 1.1,
        cd_min: 0.012,
        cd_second_order_factor: 0.6,
        cd_max_after_stall: 1.3,
        cd_power_after_stall: 1.8,

        mean_positive_stall_angle: Float::to_radians(16.0),
        mean_negative_stall_angle: Float::to_radians(24.0),
        stall_range: Float::to_radians(5.0),


    }
}

fn test_angles() -> Vec<Float> {
    (-90..=90).map(|angle| Float::from(angle as f32).to_radians()).collect()
}

fn assert_same_coefficients(foil_a: &Foil, foil_b: &Foil) {
    for angle in test_angles() {
        assert!((foil_a.lift_coefficient(angle) - foil_b.lift_coefficient(angle)).abs() < tolerance(1e-12, 1e-6));
        assert!((foil_a.drag_coefficient(angle) - foil_b.drag_coefficient(angle)).abs() < tolerance(1e-12, 1e-6));
    }
}

#[test]
fn mirrored_foil_is_the_mirror_image() {
    let foil = asymmetric_foil(0.8);
    let mirrored = foil.mirrored();

    // The angle of attack exactly at zero is skipped, as the model chooses the positive or
    // negative stall angle based on the sign of the angle there
    for angle in test_angles().into_iter().filter(|angle| *angle != 0.0) {
        let cl = foil.lift_coefficient(angle);
        let cd = foil.drag_coefficient(angle);

        assert!((mirrored.lift_coefficient(-angle) + cl).abs() < 1e-12);
        assert!((mirrored.drag_coefficient(-angle) - cd).abs() < 1e-12);
    }

    // Mirroring twice gives the original foil
    assert_same_coefficients(&mirrored.mirrored(), &foil);
}

#[test]
fn interpolation_matches_parameter_wise_interpolation() {
    // Two-sided data, so that the data is used directly, without mirroring
    let internal_state_data = vec![-0.4, 0.2, 0.5, 1.0];
    let foils_data = vec![
        Foil::default(),
        asymmetric_foil(0.5),
        Foil { cd_min: 0.05, ..asymmetric_foil(1.2) },
        Foil { mean_positive_stall_angle: 0.5, ..asymmetric_foil(2.0) },
    ];

    let varying_foil = VaryingFoil::new(internal_state_data.clone(), foils_data.clone()).unwrap();

    // The reference is the previous implementation, where each parameter is interpolated
    // separately. Values outside the data range are included to test the clamping.
    for internal_state in [-0.7, -0.4, 0.0, 0.1, 0.2, 0.35, 0.7, 1.0, 1.4] {
        let interpolate = |parameter: fn(&Foil) -> Float| {
            let data: Vec<Float> = foils_data.iter().map(parameter).collect();

            linear_interpolation(internal_state, &internal_state_data, &data)
        };

        let foil = varying_foil.get_foil(internal_state);

        assert!((foil.cl_zero_angle - interpolate(|f| f.cl_zero_angle)).abs() < 1e-12);
        assert!((foil.cl_initial_slope - interpolate(|f| f.cl_initial_slope)).abs() < 1e-12);
        assert!((foil.cd_min - interpolate(|f| f.cd_min)).abs() < 1e-12);
        assert!((foil.mean_positive_stall_angle - interpolate(|f| f.mean_positive_stall_angle)).abs() < 1e-12);
        assert!((foil.mean_negative_stall_angle - interpolate(|f| f.mean_negative_stall_angle)).abs() < 1e-12);
        assert!((foil.cl_slope_reduction_factor_positive - interpolate(|f| f.cl_slope_reduction_factor_positive)).abs() < 1e-12);
        assert!((foil.cd_power_after_stall - interpolate(|f| f.cd_power_after_stall)).abs() < 1e-12);
    }
}

#[test]
fn one_sided_data_is_mirrored_for_the_opposite_sign() {
    let positive_states = vec![0.0, 0.3, 0.6];
    let positive_foils = vec![Foil::default(), asymmetric_foil(1.0), asymmetric_foil(2.0)];

    let one_sided = VaryingFoil::new(positive_states.clone(), positive_foils.clone()).unwrap();

    // The same data, with the mirrored foils given explicitly for the negative internal states
    let two_sided = VaryingFoil::new(
        vec![-0.6, -0.3, 0.0, 0.3, 0.6],
        vec![
            positive_foils[2].mirrored(),
            positive_foils[1].mirrored(),
            positive_foils[0].clone(),
            positive_foils[1].clone(),
            positive_foils[2].clone(),
        ]
    ).unwrap();

    assert_eq!(one_sided.one_sided_data_sign(), Some(1.0));
    assert_eq!(two_sided.one_sided_data_sign(), None);

    for internal_state in [-0.8, -0.6, -0.45, -0.3, -0.1, 0.1, 0.45, 0.8] {
        assert_same_coefficients(
            &one_sided.get_foil(internal_state),
            &two_sided.get_foil(internal_state)
        );
    }
}

#[test]
fn single_foil_is_mirrored_for_the_opposite_sign() {
    // Typical setup with only one flap angle, which should still be usable for both directions
    let flap_angle = Float::to_radians(30.0);

    let varying_foil = VaryingFoil::new(vec![flap_angle], vec![asymmetric_foil(2.0)]).unwrap();

    assert_same_coefficients(&varying_foil.get_foil(flap_angle), &asymmetric_foil(2.0));
    assert_same_coefficients(&varying_foil.get_foil(-flap_angle), &asymmetric_foil(2.0).mirrored());

    // Lower magnitudes use the same foil, as the data is constant outside its range
    assert_same_coefficients(&varying_foil.get_foil(-0.1), &asymmetric_foil(2.0).mirrored());
}

#[test]
fn negative_one_sided_data_is_mirrored_for_positive_states() {
    let varying_foil = VaryingFoil::new(
        vec![-0.5, 0.0],
        vec![asymmetric_foil(-1.5), Foil::default()]
    ).unwrap();

    assert_eq!(varying_foil.one_sided_data_sign(), Some(-1.0));

    assert_same_coefficients(&varying_foil.get_foil(0.5), &asymmetric_foil(-1.5).mirrored());
    assert_same_coefficients(&varying_foil.get_foil(-0.5), &asymmetric_foil(-1.5));
}

#[test]
fn two_sided_data_is_used_directly() {
    // The negative side is deliberately not the mirror image of the positive side
    let negative_foil = Foil { cl_zero_angle: -0.3, ..Foil::default() };
    let positive_foil = asymmetric_foil(1.0);

    let varying_foil = VaryingFoil::new(
        vec![-0.5, 0.5],
        vec![negative_foil.clone(), positive_foil.clone()]
    ).unwrap();

    assert_same_coefficients(&varying_foil.get_foil(-0.5), &negative_foil);
    assert_same_coefficients(&varying_foil.get_foil(0.5), &positive_foil);
}

#[test]
fn invalid_data_is_rejected() {
    assert!(VaryingFoil::new(vec![], vec![]).is_err());
    assert!(VaryingFoil::new(vec![0.0, 0.5], vec![Foil::default()]).is_err());
    assert!(VaryingFoil::new(vec![0.5, 0.0], vec![Foil::default(), Foil::default()]).is_err());
    assert!(VaryingFoil::new(vec![0.5, 0.5], vec![Foil::default(), Foil::default()]).is_err());

    // The checks are also used when reading the input from a string
    assert!(VaryingFoil::new_from_string(
        r#"{"internal_state_data": [0.5, 0.0], "foils_data": [{}, {}]}"#
    ).is_err());

    assert!(VaryingFoil::new_from_string(
        r#"{"internal_state_data": [0.0, 0.5], "foils_data": [{}, {"cl_zero_angle": 1.0}]}"#
    ).is_ok());
}
