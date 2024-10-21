use super::*;

use math_utils::spatial_vector::SpatialVector;
use crate::lifting_line::tests::test_setup::RectangularWing;

#[test]
fn wake_building() {
    let line_force_model = RectangularWing{
        nr_strips: 10,
        ..Default::default()
    }.build().build();

    let velocity = SpatialVector::<3>::new(1.2, 0.0, 0.0);

    let time_step = 0.25; 

    let dynamic_wake = WakeBuilder{
        wake_length: WakeLength::NrPanels(10),
        last_panel_relative_length: 25.0,
        ..Default::default()
    }.build(time_step, &line_force_model, velocity);

    dynamic_wake.write_wake_to_obj_file(
        "test_output/newly_built_dynamic_wake.obj"
    ).unwrap();
    
}