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
    pub internal_state_data: Option<Vec<Float>>,
    pub internal_state_set_point_type: InternalStateSetPointType,
    pub wing_internal_state_max_value: Option<Float>,
    pub use_effective_angle_of_attack: bool,
    pub max_local_wing_angle_change_rate: Option<Float>,
    pub wing_internal_state_max_change_rate: Option<Float>
}

pub enum InternalStateSetPointType {
    SectionModel,
    Wing,
}

pub struct SpanwiseMeasurementBuilder {
    pub non_dim_start_location: f64,
    pub non_dim_end_location: f64,
}
```

## Set points

The set points are given as a function of the apparent wind direction, and are linearly interpolated between the data points. The `angle_of_attack_data` gives the set points for the angle of attack, while the `internal_state_data` gives the set points for the internal state of each wing. Both are optional, and are only used if they are given. All set points are scaled by a *loading* factor, which can be used to reduce the sail forces, for instance in strong winds.

### Internal state set points
The [internal state](../sectional_models/sectional_models_intro.md) of a wing can be given in different units than the internal state of the section model on each line element. For instance, the internal state of a rotor sail wing is the revolutions per second, while the section model uses the spin ratio. The `internal_state_set_point_type` defines which units the `internal_state_data` is given in:

- `SectionModel`: The data is given in the units of the section model, and is converted to the wing internal state using the measured flow conditions. This is the default. For a rotor sail, the data is then given as the spin ratio. For a suction sail, the data is given as the suction or power coefficient, depending on the internal state conversion in the [active air flow foil model](../sectional_models/active_air_flow_foil.md). 
- `Wing`: The data is given directly as the wing internal state, and is used without any conversion. For a rotor sail, the data is then given as the revolutions per second. For a suction sail with a power based internal state conversion, the data is given as the power, where the sign sets the direction of the suction.

For section models where the wing internal state is used directly by the section model, such as for multi-element foils, the two options give the same result.

The conversion from section model units to the wing internal state assumes that the measured flow conditions are representative for the whole wing. For suction sails, the conversion from a suction coefficient to power is therefore an open loop estimate, as the suction coefficient will vary along the span in the actual simulation.

### Limits
The `wing_internal_state_max_value` limits the absolute value of the wing internal state, while keeping the sign. The `wing_internal_state_max_change_rate` limits how fast the wing internal state can change, per unit time. Both limits are applied to the wing internal state, after any conversion from the set point data. The units therefore follow the wing internal state, which depends on the section model:

- For rotor sails, the limits are given in revolutions per second, and revolutions per second per second.
- For suction sails with a power based internal state conversion, the limits are given in power, and power per second. That is, the max value is the maximum power.
- For suction sails using the `SuctionCoefficientDirect` conversion, the limits are given as a suction coefficient.

The `max_local_wing_angle_change_rate` limits how fast the local wing angle can change, in radians per second. 

## Spanwise measurement

The `SpanwiseMeasurementBuilder` specifies which part of each wing that is used when measuring the flow conditions that are used as input to the controller. The values are given as non-dimensional span distances, varying from -0.5 to 0.5 along each wing. The default values are -0.25 and 0.25, which means that the central half of each wing is used.
