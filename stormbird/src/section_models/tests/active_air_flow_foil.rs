use stormath::type_aliases::Float;

use crate::section_models::foil::Foil;
use crate::section_models::varying_foil::VaryingFoil;
use crate::section_models::active_air_flow_foil::{
    ActiveAirFlowFoil,
    InternalStateConversion,
    SegmentFlowConditions
};
use crate::section_models::active_air_flow_foil::internal_flow::{
    InternalFlow,
    InternalFlowInput,
    InternalFlowSolverSettings
};
use crate::section_models::active_air_flow_foil::fan::{Fan, FanCurves, FanCurveData};

const FAN_EFFICIENCY: Float = 0.7;
const DENSITY: Float = 1.225;

fn test_internal_flow() -> InternalFlow {
    InternalFlow {
        cp_zero_cl: -0.5,
        cp_gradient: -0.8,
        loss_coefficient: 10.0,
        porous_fraction: 0.3,
        solver_settings: InternalFlowSolverSettings::default(),
    }
}

fn power_conversion(fan_efficiency: Float) -> InternalStateConversion {
    InternalStateConversion::PowerToInternalFlow {
        internal_flow: test_internal_flow(),
        fan_efficiency,
    }
}

const FAN_REFERENCE_SPEED: Float = 20.0;

/// Dimensional fan curves at 20 RPS, where the power is consistent with a constant efficiency. A
/// dense set of data points is used, so that the interpolated power is close to the exact value.
fn test_fan_curve_data(efficiency: Float) -> FanCurveData {
    let n = 400;

    let flow_data: Vec<Float> = (0..n).map(|i| 8.0 * i as Float / (n - 1) as Float).collect();
    let pressure_data: Vec<Float> = flow_data.iter().map(|q| 600.0 - 8.0 * q * q).collect();
    let power_data: Vec<Float> = flow_data.iter().zip(&pressure_data)
        .map(|(q, p)| (q * p / efficiency).max(0.0))
        .collect();

    FanCurveData::new(flow_data, pressure_data, power_data).unwrap()
}

fn test_fan(duct_loss_coefficient: Float) -> Fan {
    Fan {
        curves: FanCurves::Dimensional {
            reference_rotational_speed: FAN_REFERENCE_SPEED,
            reference_density: DENSITY,
            data: test_fan_curve_data(FAN_EFFICIENCY),
        },
        number_of_fans: 1,
        duct_loss_coefficient,
    }
}

fn fan_conversion(fan: Fan) -> InternalStateConversion {
    InternalStateConversion::FanSpeedToInternalFlow {
        internal_flow: test_internal_flow(),
        fan,
    }
}

/// Foil model with mirrored data for negative internal states, so that the sign of the internal
/// state sets the direction of the lift
fn test_model(internal_state_conversion: InternalStateConversion) -> ActiveAirFlowFoil {
    let foil = |cl_zero_angle: Float| Foil {
        cl_zero_angle,
        ..Foil::default()
    };

    ActiveAirFlowFoil {
        varying_foil: VaryingFoil {
            internal_state_data: vec![-0.05, 0.0, 0.05],
            foils_data: vec![foil(-1.0), foil(0.0), foil(1.0)],
        },
        internal_state_conversion,
    }
}

fn test_segments() -> Vec<SegmentFlowConditions> {
    (0..5).map(|i| {
        SegmentFlowConditions {
            angle_of_attack: (5.0 + 2.0 * i as Float).to_radians(),
            velocity: 10.0 + i as Float,
            area: 1.0,
        }
    }).collect()
}

fn mirrored(segments: &[SegmentFlowConditions]) -> Vec<SegmentFlowConditions> {
    segments.iter().map(|segment| {
        SegmentFlowConditions {
            angle_of_attack: -segment.angle_of_attack,
            ..segment.clone()
        }
    }).collect()
}

fn total_volume_flow(segments: &[SegmentFlowConditions], suction_coefficients: &[Float]) -> Float {
    segments.iter().zip(suction_coefficients).map(|(segment, cq)| {
        cq.abs() * segment.velocity * segment.area
    }).sum()
}

/// Checks that the suction coefficients are consistent with the internal flow model at the given
/// internal suction
fn assert_self_consistent(
    model: &ActiveAirFlowFoil,
    internal_flow: &InternalFlow,
    segments: &[SegmentFlowConditions],
    suction_coefficients: &[Float],
    internal_suction: Float,
) {
    for (segment, &cq) in segments.iter().zip(suction_coefficients) {
        assert!(cq >= 0.0);

        let input = InternalFlowInput {
            lift_coefficient: model.lift_coefficient(segment.angle_of_attack, cq),
            segment_area: segment.area,
            density: DENSITY,
            velocity: segment.velocity,
            internal_suction,
        };

        let cq_flow_model = internal_flow.suction_coefficient(&input).max(0.0);

        assert!((cq_flow_model - cq).abs() < 1e-6);
    }
}

#[test]
fn suction_from_power_matches_power_and_is_self_consistent() {
    let internal_flow = test_internal_flow();
    let model = test_model(power_conversion(FAN_EFFICIENCY));

    let power = 500.0;

    let segments = test_segments();

    let flow_power = FAN_EFFICIENCY * power;

    let internal_suction = model.internal_suction_from_flow_power(
        &internal_flow, flow_power, &segments, DENSITY, 1.0
    );

    let suction_coefficients = model.section_internal_states(power, &segments, DENSITY);

    let computed_flow_power = internal_suction * total_volume_flow(&segments, &suction_coefficients);

    dbg!(internal_suction, &suction_coefficients, computed_flow_power);

    assert!((computed_flow_power - flow_power).abs() / flow_power < 1e-4);

    assert_self_consistent(&model, &internal_flow, &segments, &suction_coefficients, internal_suction);
}

#[test]
fn negative_power_mirrors_the_suction_direction() {
    let model = test_model(power_conversion(FAN_EFFICIENCY));

    let power = 500.0;

    let segments = test_segments();

    let positive = model.section_internal_states(power, &segments, DENSITY);
    let negative = model.section_internal_states(-power, &mirrored(&segments), DENSITY);

    for (cq_positive, cq_negative) in positive.iter().zip(&negative) {
        assert!((cq_positive + cq_negative).abs() < 1e-8);
    }
}

#[test]
fn section_model_power_equals_wing_power() {
    let segments = test_segments();

    let conversions = [
        InternalStateConversion::PowerToPowerCoefficient,
        power_conversion(FAN_EFFICIENCY),
    ];

    for conversion in conversions {
        let model = test_model(conversion);

        for power in [500.0, -500.0] {
            let internal_states = model.section_internal_states(power, &segments, DENSITY);

            let computed_power = model.wing_power(power, &segments, &internal_states, DENSITY);

            dbg!(power, computed_power);

            assert!((computed_power - power.abs()).abs() / power.abs() < 1e-4);
        }
    }
}

#[test]
fn zero_power_gives_zero_suction_coefficient() {
    let model = test_model(power_conversion(FAN_EFFICIENCY));

    let segments = vec![SegmentFlowConditions { angle_of_attack: 0.1, velocity: 10.0, area: 1.0 }];

    let suction_coefficients = model.section_internal_states(0.0, &segments, DENSITY);

    assert_eq!(suction_coefficients, vec![0.0]);
}

#[test]
fn zero_velocity_gives_zero_suction_and_no_nan() {
    let internal_flow = test_internal_flow();

    let segments = vec![SegmentFlowConditions { angle_of_attack: 0.1, velocity: 0.0, area: 1.0 }; 3];

    let model = test_model(power_conversion(FAN_EFFICIENCY));

    let internal_suction = model.internal_suction_from_flow_power(
        &internal_flow, 500.0, &segments, DENSITY, 1.0
    );

    assert_eq!(internal_suction, 0.0);
    assert_eq!(model.section_internal_states(500.0, &segments, DENSITY), vec![0.0; 3]);

    let fan_model = test_model(fan_conversion(test_fan(0.0)));

    assert_eq!(fan_model.section_internal_states(20.0, &segments, DENSITY), vec![0.0; 3]);
}

#[test]
fn suction_coefficient_direct_is_passed_through_with_zero_power() {
    let model = test_model(InternalStateConversion::SuctionCoefficientDirect);

    let segments = test_segments();

    let internal_states = model.section_internal_states(-0.02, &segments, DENSITY);

    assert_eq!(internal_states, vec![-0.02; segments.len()]);

    assert_eq!(model.wing_power(-0.02, &segments, &internal_states, DENSITY), 0.0);
}

#[test]
fn power_to_power_coefficient_uses_wing_area_and_local_velocity() {
    let model = test_model(InternalStateConversion::PowerToPowerCoefficient);

    let power = 500.0;

    let segments = test_segments();

    let wing_area: Float = segments.iter().map(|segment| segment.area).sum();

    let internal_states = model.section_internal_states(power, &segments, DENSITY);

    for (segment, &cp) in segments.iter().zip(&internal_states) {
        let expected = power / (0.5 * wing_area * DENSITY * segment.velocity.powi(3));

        assert!((cp - expected).abs() < 1e-12);
    }
}

/// With uniform flow conditions over the wing, the conversion from section state to wing state
/// should be the exact inverse of the conversion from wing state to section state.
#[test]
fn wing_state_from_section_state_inverts_section_internal_states() {
    let angle_of_attack = (8.0 as Float).to_radians();
    let velocity = 12.0;

    let segments: Vec<SegmentFlowConditions> = (0..4).map(|_| {
        SegmentFlowConditions { angle_of_attack, velocity, area: 1.5 }
    }).collect();

    let wing_area: Float = segments.iter().map(|segment| segment.area).sum();

    let cases = [
        (InternalStateConversion::SuctionCoefficientDirect, 0.03),
        (InternalStateConversion::PowerToPowerCoefficient, 500.0),
        (power_conversion(FAN_EFFICIENCY), 500.0),
        (fan_conversion(test_fan(0.0)), 15.0),
        (fan_conversion(test_fan(5.0)), 15.0),
    ];

    for (conversion, wing_state_magnitude) in cases {
        let model = test_model(conversion);

        for wing_state in [wing_state_magnitude, -wing_state_magnitude] {
            let section_state = model.section_internal_states(wing_state, &segments, DENSITY)[0];

            let wing_state_back = model.wing_state_from_section_state(
                section_state, angle_of_attack, velocity, wing_area, DENSITY
            );

            dbg!(&model.internal_state_conversion, wing_state, section_state, wing_state_back);

            assert!((wing_state_back - wing_state).abs() / wing_state.abs() < 1e-4);
        }
    }
}

/// The fan efficiency should only scale the power delivered to the flow
#[test]
fn fan_efficiency_scales_the_flow_power() {
    let power = 500.0;

    let segments = test_segments();

    let model = test_model(power_conversion(FAN_EFFICIENCY));
    let ideal_model = test_model(power_conversion(1.0));

    let suction_coefficients = model.section_internal_states(power, &segments, DENSITY);
    let ideal_suction_coefficients = ideal_model.section_internal_states(
        FAN_EFFICIENCY * power, &segments, DENSITY
    );

    for (cq, cq_ideal) in suction_coefficients.iter().zip(&ideal_suction_coefficients) {
        assert!((cq - cq_ideal).abs() < 1e-8);
    }
}

/// At the operating point, the pressure rise over the fan should balance the internal suction and
/// the duct losses, and the suction coefficients should be consistent with the internal flow model
#[test]
fn fan_operating_point_balances_the_pressure() {
    let internal_flow = test_internal_flow();
    let segments = test_segments();

    for duct_loss_coefficient in [0.0, 5.0] {
        let fan = test_fan(duct_loss_coefficient);
        let model = test_model(fan_conversion(fan.clone()));

        let rotational_speed = 15.0;

        let internal_suction = model.internal_suction_from_fan_speed(
            &internal_flow, &fan, rotational_speed, &segments, DENSITY, 1.0
        );

        let suction_coefficients = model.section_internal_states(rotational_speed, &segments, DENSITY);

        let flow = total_volume_flow(&segments, &suction_coefficients);

        let pressure_rise = fan.pressure_rise(flow, rotational_speed, DENSITY);
        let duct_loss = fan.duct_loss(flow);

        dbg!(internal_suction, flow, pressure_rise, duct_loss);

        assert!(flow > 0.0);
        assert!((pressure_rise - internal_suction - duct_loss).abs() / pressure_rise < 1e-5);

        assert_self_consistent(&model, &internal_flow, &segments, &suction_coefficients, internal_suction);
    }
}

#[test]
fn negative_fan_speed_mirrors_the_suction_direction() {
    let model = test_model(fan_conversion(test_fan(0.0)));

    let segments = test_segments();

    let positive = model.section_internal_states(15.0, &segments, DENSITY);
    let negative = model.section_internal_states(-15.0, &mirrored(&segments), DENSITY);

    for (cq_positive, cq_negative) in positive.iter().zip(&negative) {
        assert!((cq_positive + cq_negative).abs() < 1e-8);
    }

    let positive_power = model.wing_power(15.0, &segments, &positive, DENSITY);
    let negative_power = model.wing_power(-15.0, &mirrored(&segments), &negative, DENSITY);

    assert!(positive_power > 0.0);
    assert!((positive_power - negative_power).abs() < 1e-8);
}

/// A fan with a constant efficiency should give the same suction coefficients as the constant
/// efficiency model, when the power is the same
#[test]
fn fan_with_constant_efficiency_matches_the_power_model() {
    let segments = test_segments();

    let fan_model = test_model(fan_conversion(test_fan(0.0)));
    let power_model = test_model(power_conversion(FAN_EFFICIENCY));

    for rotational_speed in [10.0, 15.0, 20.0] {
        let fan_suction_coefficients = fan_model.section_internal_states(
            rotational_speed, &segments, DENSITY
        );

        let fan_power = fan_model.wing_power(
            rotational_speed, &segments, &fan_suction_coefficients, DENSITY
        );

        let power_suction_coefficients = power_model.section_internal_states(
            fan_power, &segments, DENSITY
        );

        dbg!(rotational_speed, fan_power, &fan_suction_coefficients, &power_suction_coefficients);

        for (cq_fan, cq_power) in fan_suction_coefficients.iter().zip(&power_suction_coefficients) {
            assert!((cq_fan - cq_power).abs() / cq_fan.abs().max(1e-3) < 1e-3);
        }
    }
}

/// The dimensional curves should follow the fan affinity laws, and the non-dimensional curves
/// should give the same result when the data is converted
#[test]
fn fan_curves_follow_the_affinity_laws() {
    let dimensional_data = test_fan_curve_data(FAN_EFFICIENCY);

    let dimensional = FanCurves::Dimensional {
        reference_rotational_speed: FAN_REFERENCE_SPEED,
        reference_density: DENSITY,
        data: dimensional_data.clone(),
    };

    let diameter = 0.8;
    let n_ref = FAN_REFERENCE_SPEED;

    let non_dimensional = FanCurves::NonDimensional {
        diameter,
        data: FanCurveData::new(
            dimensional_data.flow_data.iter().map(|q| q / (n_ref * diameter.powi(3))).collect(),
            dimensional_data.pressure_data.iter().map(|p| p / (DENSITY * n_ref.powi(2) * diameter.powi(2))).collect(),
            dimensional_data.power_data.iter().map(|p| p / (DENSITY * n_ref.powi(3) * diameter.powi(5))).collect(),
        ).unwrap(),
    };

    let reference_flow = 3.0;
    let reference_pressure = dimensional.pressure_rise(reference_flow, n_ref, DENSITY);
    let reference_power = dimensional.power(reference_flow, n_ref, DENSITY);

    for (speed, density) in [(10.0, 1.225), (25.0, 1.0)] {
        let speed_ratio: Float = speed / n_ref;
        let density_ratio = density / DENSITY;

        let flow = reference_flow * speed_ratio;

        let expected_pressure = reference_pressure * density_ratio * speed_ratio.powi(2);
        let expected_power = reference_power * density_ratio * speed_ratio.powi(3);

        for curves in [&dimensional, &non_dimensional] {
            let pressure = curves.pressure_rise(flow, speed, density);
            let power = curves.power(flow, speed, density);

            assert!((pressure - expected_pressure).abs() / expected_pressure < 1e-8);
            assert!((power - expected_power).abs() / expected_power < 1e-8);
        }
    }
}

/// Multiple fans in parallel should share the flow equally
#[test]
fn fans_in_parallel_share_the_flow() {
    let single_data = test_fan_curve_data(FAN_EFFICIENCY);

    let double_fan = Fan {
        curves: FanCurves::Dimensional {
            reference_rotational_speed: FAN_REFERENCE_SPEED,
            reference_density: DENSITY,
            data: FanCurveData::new(
                single_data.flow_data.iter().map(|q| 0.5 * q).collect(),
                single_data.pressure_data.clone(),
                single_data.power_data.iter().map(|p| 0.5 * p).collect(),
            ).unwrap(),
        },
        number_of_fans: 2,
        duct_loss_coefficient: 0.0,
    };

    let segments = test_segments();

    let single_model = test_model(fan_conversion(test_fan(0.0)));
    let double_model = test_model(fan_conversion(double_fan));

    let single = single_model.section_internal_states(15.0, &segments, DENSITY);
    let double = double_model.section_internal_states(15.0, &segments, DENSITY);

    for (cq_single, cq_double) in single.iter().zip(&double) {
        assert!((cq_single - cq_double).abs() < 1e-8);
    }

    let single_power = single_model.wing_power(15.0, &segments, &single, DENSITY);
    let double_power = double_model.wing_power(15.0, &segments, &double, DENSITY);

    assert!((single_power - double_power).abs() / single_power < 1e-8);
}

/// A fan that can not overcome the external suction should give no flow
#[test]
fn weak_fan_gives_no_flow() {
    let segments = test_segments();

    let model = test_model(fan_conversion(test_fan(0.0)));

    // The maximum pressure rise at 1 RPS is 600 Pa / 400 = 1.5 Pa, which is less than the
    // external suction on all segments
    let suction_coefficients = model.section_internal_states(1.0, &segments, DENSITY);

    assert_eq!(suction_coefficients, vec![0.0; segments.len()]);

    assert_eq!(model.section_internal_states(0.0, &segments, DENSITY), vec![0.0; segments.len()]);
    assert_eq!(model.wing_power(0.0, &segments, &suction_coefficients, DENSITY), 0.0);
}

#[test]
fn fan_curves_from_json() {
    let valid = r#"{
        "curves": {
            "NonDimensional": {
                "diameter": 0.8,
                "data": {
                    "flow_data": [0.0, 0.5, 1.0],
                    "pressure_data": [5.0, 4.0, 2.0],
                    "power_data": [0.5, 2.0, 3.0]
                }
            }
        },
        "number_of_fans": 2
    }"#;

    let fan: Fan = serde_json::from_str(valid).unwrap();

    assert_eq!(fan.number_of_fans, 2);
    assert_eq!(fan.duct_loss_coefficient, 0.0);

    // Pressure increasing with the flow, which is the unstable region of the fan
    let unstable = valid.replace("[5.0, 4.0, 2.0]", "[5.0, 6.0, 2.0]");

    let error = serde_json::from_str::<Fan>(&unstable).unwrap_err();

    assert!(error.to_string().contains("strictly decreasing"));
}
