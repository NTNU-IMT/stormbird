use stormath::type_aliases::Float;

use crate::line_force_model::wing_internal_state::{SpanVariation, WingInternalState, InternalCompartments};

fn single_compartment() -> InternalCompartments {
    InternalCompartments::default()
}

fn assert_close(a: Float, b: Float) {
    assert!((a - b).abs() < 1e-12, "{} != {}", a, b);
}

#[test]
fn linear_span_variation() {
    let variation = SpanVariation::new_linear(vec![-0.25, 0.25], vec![1.0, 3.0]).unwrap();

    // Interpolated between the locations, and constant outside
    assert_close(variation.value_at(0.0), 2.0);
    assert_close(variation.value_at(-0.5), 1.0);
    assert_close(variation.value_at(0.5), 3.0);

    // Exact integral: 0.25 * 1 + 0.5 * 2 + 0.25 * 3
    assert_close(variation.integral(-0.5, 0.5), 2.0);
    assert_close(variation.mean(-0.5, 0.5), 2.0);

    // Integral over a part of the span that starts between two locations
    assert_close(variation.integral(0.0, 0.5), 0.25 * 2.5 + 0.25 * 3.0);
}

#[test]
fn piecewise_constant_span_variation() {
    let variation = SpanVariation::new_piecewise_constant(
        vec![-0.5, 0.0, 0.5], vec![10.0, 20.0]
    ).unwrap();

    assert_close(variation.value_at(-0.25), 10.0);
    assert_close(variation.value_at(0.25), 20.0);

    // A boundary belongs to the zone above it, and values outside the boundaries use the closest
    // zone
    assert_close(variation.value_at(0.0), 20.0);
    assert_close(variation.value_at(-0.6), 10.0);
    assert_close(variation.value_at(0.6), 20.0);

    // The mean over a zone is exactly the value in the zone
    assert_close(variation.mean(-0.5, 0.0), 10.0);
    assert_close(variation.mean(0.0, 0.5), 20.0);

    // Integral across the boundary
    assert_close(variation.integral(-0.25, 0.25), 0.25 * 10.0 + 0.25 * 20.0);
}

#[test]
fn integral_of_absolute_value_with_sign_change() {
    let variation = SpanVariation::new_linear(vec![-0.5, 0.5], vec![-1.0, 1.0]).unwrap();

    assert_close(variation.integral(-0.5, 0.5), 0.0);
    assert_close(variation.integral_of_absolute_value(-0.5, 0.5), 0.5);
}

#[test]
fn wing_internal_state_span_integral() {
    let span_length = 30.0;

    // A constant value per span length, integrated over the whole span, gives the value times the
    // span length
    let constant = WingInternalState::Constant(100.0);

    assert_close(constant.span_integral(-0.5, 0.5, span_length), 3000.0);
    assert_close(constant.representative_value(), 100.0);

    let varying = WingInternalState::SpanVarying(
        SpanVariation::new_piecewise_constant(vec![-0.5, 0.0, 0.5], vec![100.0, -300.0]).unwrap()
    );

    assert_close(varying.span_integral(-0.5, 0.5, span_length), -3000.0);
    assert_close(varying.span_integral_of_absolute_value(-0.5, 0.5, span_length), 6000.0);
    assert_close(varying.representative_value(), -100.0);
    assert_close(varying.representative_absolute_value(), 200.0);
}

#[test]
fn invalid_span_variations_are_rejected() {
    assert!(SpanVariation::new_linear(vec![0.0, 0.0], vec![1.0, 2.0]).is_err());
    assert!(SpanVariation::new_linear(vec![-0.5, 0.5], vec![1.0]).is_err());
    assert!(SpanVariation::new_linear(vec![], vec![]).is_err());

    assert!(SpanVariation::new_piecewise_constant(vec![-0.5, 0.5], vec![1.0, 2.0]).is_err());
    assert!(SpanVariation::new_piecewise_constant(vec![0.5, -0.5], vec![1.0]).is_err());

    let valid = r#"{"SpanVarying": {"Linear": {"locations": [-0.5, 0.5], "values": [1.0, 2.0]}}}"#;
    let state: WingInternalState = serde_json::from_str(valid).unwrap();

    assert_close(state.value_at(0.0), 1.5);

    let invalid = r#"{"SpanVarying": {"Linear": {"locations": [0.5, -0.5], "values": [1.0, 2.0]}}}"#;

    assert!(serde_json::from_str::<WingInternalState>(invalid).is_err());
}

mod section_models {
    use super::*;

    use crate::section_models::{SectionModel, SegmentFlowConditions};
    use crate::section_models::foil::Foil;
    use crate::section_models::varying_foil::VaryingFoil;
    use crate::section_models::rotating_cylinder::RotatingCylinder;

    fn segments() -> Vec<SegmentFlowConditions> {
        (0..4).map(|i| {
            SegmentFlowConditions {
                angle_of_attack: 0.1,
                velocity: 10.0,
                area: 2.0 * 1.5,
                span_length: 1.5,
                non_dim_span_location: -0.375 + 0.25 * i as Float,
                is_virtual: false,
                compartment_index: 0,
            }
        }).collect()
    }

    fn linear_state() -> WingInternalState {
        WingInternalState::SpanVarying(
            SpanVariation::new_linear(vec![-0.5, 0.5], vec![0.0, 0.4]).unwrap()
        )
    }

    #[test]
    fn multi_element_foil_uses_the_local_value() {
        let model = SectionModel::MultiElementFoil(VaryingFoil {
            internal_state_data: vec![0.0, 0.5],
            foils_data: vec![Foil::default(), Foil::default()],
        });

        let segments = segments();
        let state = linear_state();

        let section_states = model.section_internal_states(&state, &segments, &single_compartment(), 1.225);

        for (segment, section_state) in segments.iter().zip(section_states) {
            assert_close(section_state, state.value_at(segment.non_dim_span_location));
        }
    }

    #[test]
    fn rotating_cylinder_uses_the_local_rotational_speed() {
        let model = SectionModel::RotatingCylinder(RotatingCylinder::default());

        let segments = segments();
        let state = linear_state();

        let section_states = model.section_internal_states(&state, &segments, &single_compartment(), 1.225);

        for (segment, section_state) in segments.iter().zip(section_states) {
            let expected = RotatingCylinder::spin_ratio_from_revolutions_per_second(
                state.value_at(segment.non_dim_span_location), 
                2.0, 
                segment.velocity
            );

            assert_close(section_state, expected);
        }
    }
}
