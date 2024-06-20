use super::*;

use crate::viscous_wakes::wake_models;

#[test]
fn test_schlichting_correction_factor() {

    let wake_coordinates = WakeCoordinates {
        velocity: 1.0,
        normal: 0.2,
    };

    let wake_model_parameters = WakeModelParameters {
        drag_coefficient: 0.5,
        width: 1.0,
    };

    let value_from_previous_implementation = 0.7068139379676727;

    let correction_factor = wake_models::schlichting_correction_factor(&wake_coordinates, &wake_model_parameters);

    assert!((correction_factor - value_from_previous_implementation).abs() < 1e-6);

    dbg!(&correction_factor);
}