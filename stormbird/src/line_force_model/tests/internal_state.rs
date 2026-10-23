use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

use crate::line_force_model::input_power::InputPowerModel;
use crate::line_force_model::wing_internal_state::{SpanVariation, WingInternalState};

use super::get_example_model;

/// The internal state of the section models should follow the span variation, evaluated at the
/// control point of each line element
#[test]
fn span_varying_internal_state() {
    let mut model = get_example_model();

    let span_variation = SpanVariation::new_linear(vec![-0.5, 0.5], vec![-1.0, 1.0]).unwrap();

    model.set_internal_state(&[0.3, 0.3]);
    model.set_internal_state_span_variation(1, span_variation.clone());

    let velocity = vec![SpatialVector::from([10.0, 0.0, 0.0]); model.nr_span_lines()];

    let section_states = model.section_models_internal_state(&velocity);

    for (index, &section_state) in section_states.iter().enumerate().take(model.nr_span_lines()) {
        let expected = if model.wing_index_from_global(index) == 0 {
            0.3
        } else {
            span_variation.value_at(model.ctrl_point_spanwise_distance_non_dimensional[index])
        };

        assert!((section_state - expected).abs() < 1e-12);
    }

    // The values vary along the span of the second wing
    let second_wing = model.wing_indices[1].clone();

    assert!(section_states[second_wing.start] < 0.0);
    assert!(section_states[second_wing.end - 1] > 0.0);

    // Setting the internal state with float values gives constant values again
    model.set_internal_state(&[0.3, 0.5]);

    assert!(model.internal_state[1].is_constant());
    assert!((model.internal_state[1].representative_value() - 0.5).abs() < 1e-12);
}

/// The direct input power model integrates a span varying power per span length
#[test]
fn direct_input_power_with_span_variation() {
    let span_length: Float = 30.0;

    let state = WingInternalState::SpanVarying(
        SpanVariation::new_piecewise_constant(vec![-0.5, 0.0, 0.5], vec![100.0, -200.0]).unwrap()
    );

    let power = InputPowerModel::DirectFromInternalState.input_power(&state, span_length, || 0.0);

    assert!((power - (100.0 * 15.0 + 200.0 * 15.0)).abs() < 1e-9);

    let constant_power = InputPowerModel::DirectFromInternalState.input_power(
        &WingInternalState::Constant(-500.0), span_length, || 0.0
    );

    assert!((constant_power - 500.0).abs() < 1e-12);
}

mod compartments {
    use crate::line_force_model::builder::single_wing::WingBuilder;
    use crate::line_force_model::builder::LineForceModelBuilder;
    use crate::line_force_model::LineForceModel;
    use crate::section_models::SectionModel;
    use crate::section_models::foil::Foil;
    use crate::section_models::varying_foil::VaryingFoil;
    use crate::section_models::active_air_flow_foil::{ActiveAirFlowFoil, InternalStateConversion};
    use crate::section_models::active_air_flow_foil::internal_flow::InternalFlow;

    use stormath::spatial_vector::SpatialVector;
    use stormath::type_aliases::Float;

    fn suction_sail_section_model() -> SectionModel {
        SectionModel::ActiveAirFlowFoil(ActiveAirFlowFoil {
            varying_foil: VaryingFoil {
                internal_state_data: vec![0.0, 0.05],
                foils_data: vec![Foil::default(), Foil { cl_zero_angle: 1.0, ..Foil::default() }],
            },
            internal_state_conversion: InternalStateConversion::PowerToInternalFlow {
                internal_flow: InternalFlow::new(
                    -0.5, -20.0, 10.0, 0.3, Default::default()
                ).unwrap(),
                fan_efficiency: 0.7,
            },
        })
    }

    fn build_model(section_model: SectionModel, boundaries: Option<Vec<Float>>) -> LineForceModel {
        let mut builder = LineForceModelBuilder::new(8);

        builder.add_wing(WingBuilder {
            section_points: vec![
                SpatialVector::from([0.0, 0.0, 0.0]),
                SpatialVector::from([0.0, 0.0, 20.0]),
            ],
            chord_vectors: vec![SpatialVector::from([5.0, 0.0, 0.0]); 2],
            section_model,
            internal_compartment_boundaries: boundaries,
            ..Default::default()
        });

        builder.build()
    }

    #[test]
    fn segments_are_assigned_to_compartments() {
        let model = build_model(suction_sail_section_model(), Some(vec![-0.5, 0.0, 0.5]));

        assert_eq!(model.internal_compartments[0].nr_compartments(), 2);

        let nr = model.nr_span_lines();

        let segments = model.wing_segment_flow_conditions(0, &vec![0.1; nr], &vec![10.0; nr]);

        let indices: Vec<usize> = segments.iter().map(|segment| segment.compartment_index).collect();

        assert_eq!(indices, vec![0, 0, 0, 0, 1, 1, 1, 1]);
    }

    #[test]
    fn default_is_a_single_compartment() {
        let model = build_model(suction_sail_section_model(), None);

        assert_eq!(model.internal_compartments[0].nr_compartments(), 1);
    }

    #[test]
    #[should_panic(expected = "only used by section models with an internal flow")]
    fn compartments_are_rejected_for_other_section_models() {
        build_model(SectionModel::SingleElementFoil(Foil::default()), Some(vec![-0.5, 0.0, 0.5]));
    }

    #[test]
    #[should_panic(expected = "must start at")]
    fn compartments_must_cover_the_whole_span() {
        build_model(suction_sail_section_model(), Some(vec![-0.4, 0.0, 0.5]));
    }
}
