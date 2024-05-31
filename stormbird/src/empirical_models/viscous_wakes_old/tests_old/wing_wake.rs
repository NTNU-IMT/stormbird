use super::*;

use crate::viscous_wakes::wing_wake::WingWake;
use crate::line_force_model::span_line::SpanLine;
use spatialvector::vec3::Vec3;

use std::matches;

#[test]
fn test_wake_state_of_point_direction() {
    let height = 2.0;
    let chord_length = 1.0;

    let distance_between_wings = 2.0 * chord_length;

    let wing_front = WingWake{
        span_line: SpanLine{
            start_point: Vec3::new(-0.5 * distance_between_wings, 0.0, 0.0),
            end_point:  Vec3::new(0.0, 0.0, height),
        },
        chord_vector: Vec3::new(0.0, chord_length, 0.0),
    };

    let wing_back = WingWake{
        span_line: SpanLine{
            start_point: Vec3::new(0.5 * distance_between_wings, 0.0, 0.0),
            end_point:  Vec3::new(0.0, 0.0, height),
        },
        chord_vector: Vec3::new(0.0, chord_length, 0.0),
    };

    let wind_direections_deg: Vec<f64> = vec![0.0, 22.5, 45.0];

    for wind_direction_deg in wind_direections_deg {
        let wind_direction = wind_direction_deg.to_radians();

        let velocity = Vec3{
            x: wind_direction.cos(),
            y: wind_direction.sin(),
            z: 0.0,
        };

        let wake_state_back = wing_front.wake_state_of_point(wing_back.span_line.ctrl_point(), velocity);
        let wake_state_front = wing_back.wake_state_of_point(wing_front.span_line.ctrl_point(), velocity);

        dbg!(&wake_state_back, &wake_state_front);

        assert!(matches!(wake_state_back, WakeState::InWake(_)));
        assert!(matches!(wake_state_front, WakeState::NotInWake));
    }    
}

#[test]
fn test_wake_state_of_point_span_independence() {
    let height = 2.0;
    let chord_length = 1.0;

    let wake = WingWake{
        span_line: SpanLine{
            start_point: Vec3::new(0.0, 0.0, 0.0),
            end_point:  Vec3::new(0.0, 0.0, height),
        },
        chord_vector: Vec3::new(0.0, chord_length, 0.0),
    };

    let point_1 = Vec3::new(1.2, 0.0, 0.1 * height);
    let point_2 = Vec3::new(1.2, 0.0, 0.5 * height);

    let velocity = Vec3::new(1.0, 0.0, 0.0);

    let wake_state_1 = wake.wake_state_of_point(point_1, velocity);
    let wake_state_2 = wake.wake_state_of_point(point_2, velocity);

    if let (WakeState::InWake(wake_coordinates_1) , WakeState::InWake(wake_coordinates_2)) = (&wake_state_1, &wake_state_2) {
        assert!(wake_coordinates_1.velocity == wake_coordinates_2.velocity);
        assert!(wake_coordinates_1.normal == wake_coordinates_2.normal);
    } else {
        panic!("Points not in wake");
    }

    dbg!(&wake_state_1, &wake_state_2);
}