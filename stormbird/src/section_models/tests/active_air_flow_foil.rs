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

fn test_internal_flow() -> InternalFlow {
    InternalFlow {
        cp_zero_cl: -0.5,
        cp_gradient: -0.8,
        loss_coefficient: 10.0,
        porous_fraction: 0.3,
        fan_efficiency: 0.7,
        solver_settings: InternalFlowSolverSettings::default(),
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

/// Power integrated from the section model power coefficients
fn power_from_section_models(
    model: &ActiveAirFlowFoil,
    segments: &[SegmentFlowConditions],
    internal_states: &[Float],
    density: Float
) -> Float {
    segments.iter().zip(internal_states).map(|(segment, &internal_state)| {
        model.power_coefficient(segment.angle_of_attack, internal_state) *
            0.5 * density * segment.area * segment.velocity.powi(3)
    }).sum()
}

#[test]
fn suction_from_power_matches_power_and_is_self_consistent() {
    let internal_flow = test_internal_flow();
    let model = test_model(InternalStateConversion::PowerToInternalFlow(internal_flow.clone()));

    let density = 1.225;
    let power = 500.0;

    let segments = test_segments();

    let internal_suction = model.internal_suction_from_power(
        &internal_flow, power, &segments, density, 1.0
    );

    let suction_coefficients = model.section_internal_states(power, &segments, density);

    let total_flow: Float = segments.iter().zip(&suction_coefficients).map(|(segment, cq)| {
        cq * segment.velocity * segment.area
    }).sum();

    let computed_flow_power = internal_suction * total_flow;
    let expected_flow_power = internal_flow.flow_power_from_fan_power(power);

    dbg!(internal_suction, &suction_coefficients, computed_flow_power);

    assert!((computed_flow_power - expected_flow_power).abs() / expected_flow_power < 1e-4);

    for (segment, &cq) in segments.iter().zip(&suction_coefficients) {
        assert!(cq >= 0.0);

        let input = InternalFlowInput {
            lift_coefficient: model.lift_coefficient(segment.angle_of_attack, cq),
            segment_area: segment.area,
            density,
            velocity: segment.velocity,
            internal_suction,
        };

        let cq_flow_model = internal_flow.suction_coefficient(&input).max(0.0);

        assert!((cq_flow_model - cq).abs() < 1e-6);
    }
}

#[test]
fn negative_power_mirrors_the_suction_direction() {
    let model = test_model(InternalStateConversion::PowerToInternalFlow(test_internal_flow()));

    let density = 1.225;
    let power = 500.0;

    let segments = test_segments();

    let mirrored_segments: Vec<SegmentFlowConditions> = segments.iter().map(|segment| {
        SegmentFlowConditions {
            angle_of_attack: -segment.angle_of_attack,
            ..segment.clone()
        }
    }).collect();

    let positive = model.section_internal_states(power, &segments, density);
    let negative = model.section_internal_states(-power, &mirrored_segments, density);

    for (cq_positive, cq_negative) in positive.iter().zip(&negative) {
        assert!((cq_positive + cq_negative).abs() < 1e-8);
    }
}

#[test]
fn section_model_power_equals_wing_power() {
    let density = 1.225;
    let segments = test_segments();

    let conversions = [
        InternalStateConversion::PowerToPowerCoefficient,
        InternalStateConversion::PowerToInternalFlow(test_internal_flow()),
    ];

    for conversion in conversions {
        let model = test_model(conversion);

        for power in [500.0, -500.0] {
            let internal_states = model.section_internal_states(power, &segments, density);

            let computed_power = power_from_section_models(
                &model, &segments, &internal_states, density
            );

            dbg!(power, computed_power);

            assert!((computed_power - power.abs()).abs() / power.abs() < 1e-4);
        }
    }
}

#[test]
fn zero_power_gives_zero_suction_coefficient() {
    let model = test_model(InternalStateConversion::PowerToInternalFlow(test_internal_flow()));

    let segments = vec![SegmentFlowConditions { angle_of_attack: 0.1, velocity: 10.0, area: 1.0 }];

    let suction_coefficients = model.section_internal_states(0.0, &segments, 1.225);

    assert_eq!(suction_coefficients, vec![0.0]);
}

#[test]
fn zero_velocity_gives_zero_suction_and_no_nan() {
    let internal_flow = test_internal_flow();
    let model = test_model(InternalStateConversion::PowerToInternalFlow(internal_flow.clone()));

    let segments = vec![SegmentFlowConditions { angle_of_attack: 0.1, velocity: 0.0, area: 1.0 }; 3];

    let internal_suction = model.internal_suction_from_power(
        &internal_flow, 500.0, &segments, 1.225, 1.0
    );

    assert_eq!(internal_suction, 0.0);
    assert_eq!(model.section_internal_states(500.0, &segments, 1.225), vec![0.0; 3]);
}

#[test]
fn suction_coefficient_direct_is_passed_through_with_zero_power() {
    let model = test_model(InternalStateConversion::SuctionCoefficientDirect);

    let segments = test_segments();

    let internal_states = model.section_internal_states(-0.02, &segments, 1.225);

    assert_eq!(internal_states, vec![-0.02; segments.len()]);

    assert_eq!(power_from_section_models(&model, &segments, &internal_states, 1.225), 0.0);
}

#[test]
fn power_to_power_coefficient_uses_wing_area_and_local_velocity() {
    let model = test_model(InternalStateConversion::PowerToPowerCoefficient);

    let density = 1.225;
    let power = 500.0;

    let segments = test_segments();

    let wing_area: Float = segments.iter().map(|segment| segment.area).sum();

    let internal_states = model.section_internal_states(power, &segments, density);

    for (segment, &cp) in segments.iter().zip(&internal_states) {
        let expected = power / (0.5 * wing_area * density * segment.velocity.powi(3));

        assert!((cp - expected).abs() < 1e-12);
    }
}

/// With uniform flow conditions over the wing, the conversion from section state to wing state
/// should be the exact inverse of the conversion from wing state to section state.
#[test]
fn wing_state_from_section_state_inverts_section_internal_states() {
    let density = 1.225;
    let angle_of_attack = (8.0 as Float).to_radians();
    let velocity = 12.0;

    let segments: Vec<SegmentFlowConditions> = (0..4).map(|_| {
        SegmentFlowConditions { angle_of_attack, velocity, area: 1.5 }
    }).collect();

    let wing_area: Float = segments.iter().map(|segment| segment.area).sum();

    let conversions = [
        InternalStateConversion::SuctionCoefficientDirect,
        InternalStateConversion::PowerToPowerCoefficient,
        InternalStateConversion::PowerToInternalFlow(test_internal_flow()),
    ];

    for conversion in conversions {
        let model = test_model(conversion);

        for wing_state in [500.0, -500.0] {
            let section_state = model.section_internal_states(wing_state, &segments, density)[0];

            let wing_state_back = model.wing_state_from_section_state(
                section_state, angle_of_attack, velocity, wing_area, density
            );

            dbg!(&model.internal_state_conversion, wing_state, section_state, wing_state_back);

            assert!((wing_state_back - wing_state).abs() / wing_state.abs() < 1e-4);
        }
    }
}

/// The fan efficiency should only scale the power delivered to the flow
#[test]
fn fan_efficiency_scales_the_flow_power() {
    let density = 1.225;
    let power = 500.0;

    let segments = test_segments();

    let internal_flow = test_internal_flow();

    let ideal_internal_flow = InternalFlow {
        fan_efficiency: 1.0,
        ..internal_flow.clone()
    };

    let model = test_model(InternalStateConversion::PowerToInternalFlow(internal_flow.clone()));
    let ideal_model = test_model(InternalStateConversion::PowerToInternalFlow(ideal_internal_flow));

    let suction_coefficients = model.section_internal_states(power, &segments, density);
    let ideal_suction_coefficients = ideal_model.section_internal_states(
        internal_flow.fan_efficiency * power, &segments, density
    );

    for (cq, cq_ideal) in suction_coefficients.iter().zip(&ideal_suction_coefficients) {
        assert!((cq - cq_ideal).abs() < 1e-8);
    }
}
