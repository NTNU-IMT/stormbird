# Control system

NOTE: MORE TO COME ON THIS LATER.

The library contains different control systems that can be used to automatically adjust the sail settings for different wind conditions. The goal is to have a system that covers most use cases, while still being relatively simple. The controller can automatically be activated for different versions of the Stormbird library. However, it is also usually possible to run custom control systems in various ways with the different APIs.

The structures used to set up a control system are shown below:

```rust
pub struct ControllerBuilder {
    pub set_points: Vec<ControllerSetPoints>,
    pub spanwise_measurement: SpanwiseMeasurementBuilder,
    pub time_steps_between_updates: usize,
    pub start_time: Float,
    pub moving_average_window_size: Option<usize>,
    pub use_input_velocity_for_apparent_wind_direction: bool,
}

pub struct ControllerSetPoints {
    pub apparent_wind_directions_data: Vec<Float>,
    pub angle_of_attack_data: Option<Vec<Float>>,
    pub section_model_internal_state_data: Option<Vec<Float>>,
    pub internal_state_type: InternalStateType,
    pub use_effective_angle_of_attack: bool,
    pub max_local_wing_angle_change_rate: Option<Float>,
    pub max_internal_section_state_change_rate: Option<Float>
}

pub enum InternalStateType {
    Generic,
    SpinRatio(SpinRatioConversion),
}

pub struct SpinRatioConversion {
    diameter: f64,
    max_rps: f64,
}

pub struct SpanwiseMeasurementBuilder {
    pub non_dim_start_location: f64,
    pub non_dim_end_location: f64,
}
```

The `SpanwiseMeasurementBuilder` specifies which part of each wing that is used when measuring the flow conditions that are used as input to the controller. The values are given as non-dimensional span distances, varying from -0.5 to 0.5 along each wing. The default values are -0.25 and 0.25, which means that the central half of each wing is used.
