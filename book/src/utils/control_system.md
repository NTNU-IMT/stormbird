# Control system

NOTE: MORE TO COME ON THIS LATER.

The library contains a control system that can be used to automatically adjust the sail settings for different wind conditions. The goal is to have a system that covers most use cases, while still being relatively simple. The controller can automatically be activated for different versions of the Stormbird library. However, it is also usually possible to run custom control systems in various ways with the different APIs.

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
    pub non_dim_start_location: Float,
    pub non_dim_end_location: Float,
}
```

There is one `ControllerSetPoints` for each wing. Each time the controller is updated, it measures the flow conditions on each wing, as described in the [spanwise measurement](#spanwise-measurement) section, and uses the measured values, together with the set points, to compute a new local wing angle and a new internal state for each wing.

## Set points

The set points are given as a function of the apparent wind direction, and are linearly interpolated between the data points. The `angle_of_attack_data` gives the set points for the angle of attack, while the `internal_state_data` gives the set points for the internal state of each wing. Both are optional. If the angle of attack data is not given, the controller sets the local wing angle to zero, and if the internal state data is not given, the controller sets the internal state to zero. All set points are scaled by a *loading* factor, which can be used to reduce the sail forces, for instance in strong winds.

### Angle of attack set points
The angle of attack set point is reached by rotating each wing around its span axis, i.e., by changing the local wing angle. The `use_effective_angle_of_attack` setting defines how the local wing angle is computed:

- `false` (default): The local wing angle is set from the *geometric* angle of attack. That is, the local wing angle is the measured apparent wind direction minus the angle of attack set point. This gives the right angle of attack when the measured velocity has the same direction as the apparent wind, but does not account for the induced velocities.
- `true`: The local wing angle is set from the *effective* angle of attack. That is, the difference between the measured angle of attack and the set point is added to the current local wing angle. As the measured angle of attack includes the induced velocities, the controller then adjusts the wing until the angle of attack felt by the wing matches the set point. As the induced velocities change when the wing rotates, this requires several updates of the controller to converge.

### Internal state set points
The [internal state](../sectional_models/sectional_models_intro.md) of a wing can be given in different units than the internal state of the section model on each line element. For instance, the internal state of a rotor sail wing is the revolutions per second, while the section model uses the spin ratio. The `internal_state_set_point_type` defines which units the `internal_state_data` is given in:

- `SectionModel`: The data is given in the units of the section model, and is converted to the wing internal state using the measured flow conditions. This is the default. For a rotor sail, the data is then given as the spin ratio. For a suction sail, the data is given as the suction or power coefficient, depending on the internal state conversion in the [active air flow foil model](../sectional_models/active_air_flow_foil.md). 
- `Wing`: The data is given directly as the wing internal state, and is used without any conversion. For a rotor sail, the data is then given as the revolutions per second. For a suction sail with a power based internal state conversion, the data is given as the power, where the sign sets the direction of the suction. For a suction sail where the fans are modeled with fan curves, the data is given as the rotational speed of the fans.

For section models where the wing internal state is used directly by the section model, such as for multi-element foils, the two options give the same result.

The conversion from section model units to the wing internal state assumes that the measured flow conditions are representative for the whole wing. For suction sails, the conversion from a suction coefficient to power is therefore an open loop estimate, as the suction coefficient will vary along the span in the actual simulation.

### Limits
The `wing_internal_state_max_value` limits the absolute value of the wing internal state, while keeping the sign. The `wing_internal_state_max_change_rate` limits how fast the wing internal state can change, per unit time. Both limits are applied to the wing internal state, after any conversion from the set point data. The units therefore follow the wing internal state, which depends on the section model:

- For rotor sails, the limits are given in revolutions per second, and revolutions per second per second.
- For suction sails with a power based internal state conversion, the limits are given in power, and power per second. That is, the max value is the maximum power.
- For suction sails where the fans are modeled with fan curves, the limits are given in revolutions per second for the fans, and revolutions per second per second.
- For suction sails using the `SuctionCoefficientDirect` conversion, the limits are given as a suction coefficient.

The `max_local_wing_angle_change_rate` limits how fast the local wing angle can change, in radians per second.

The controller always sets a constant [internal state](../sectional_models/sectional_models_intro.md#constant-and-span-varying-internal-states) for each wing, which overwrites any internal state that varies along the span. Internal states that vary along the span must therefore currently be [set manually](../line_model/move_line_models.md#update-the-internal-state), without the controller. When the controller needs the current internal state of a wing with a span varying internal state, the mean value over the span is used.

## Spanwise measurement

The `SpanwiseMeasurementBuilder` specifies which part of each wing that is used when measuring the flow conditions that are used as input to the controller. The values are given as non-dimensional span distances, varying from -0.5 to 0.5 along each wing. The default values are -0.25 and 0.25, which means that the central half of each wing is used.

The measured angle of attack, velocity and apparent wind direction for each wing are the mean values over the control points in the measurement range. The start and end of the range are moved to the control points closest to the given locations, and both of these control points are included in the mean. A narrow range therefore always contains at least one control point.

By default, the apparent wind direction is measured from the velocity at the control points, including the induced velocities from the wings. If `use_input_velocity_for_apparent_wind_direction` is set to true, the input velocity to the simulation is used instead, i.e., the velocity without induced velocities. The angle of attack and the velocity magnitude are not affected by this setting.

## Update frequency
In dynamic simulations, the controller is normally called once for every time step, but only changes the sail settings every `time_steps_between_updates` time step. The default value is 1, which means that the controller updates at every time step. A larger value gives the flow time to adjust to the new sail settings before the next update, which can be useful together with the effective angle of attack, or in dynamic simulations where the wake needs time to develop.

The `start_time` gives the time when the controller starts to update the sail settings. The controller always sets the sail settings at the first time step, so that the simulation starts from a sensible state, but keeps them fixed after this until the start time is reached. This can be used to let the flow develop before the controller starts to act on it.

## Moving average of the input
By default, the controller uses the instantaneous values of the measured flow conditions. In dynamic simulations, for instance with gusts, waves or an unsteady wake, these values can vary rapidly, which can make the sail settings change more than necessary. The `moving_average_window_size` setting can be used to let the controller act on a smoothed version of the flow conditions instead.

When the window size is larger than zero, the measured angle of attack, velocity and apparent wind direction of each wing are averaged over the given number of the most recent inputs to the controller. A new input is added to the average each time the controller is called, i.e., at every time step, also when the controller does not change the sail settings. The window size is therefore given as a number of time steps, and the averaging time is the window size times the time step. Until the window is full, the average is taken over the inputs that are available. The apparent wind direction is averaged through its sine and cosine, so that the average is correct also when the direction passes ±π. The loading and the current sail settings are not averaged.

The filter is not used by default, i.e., when the window size is not given or is zero. A moving average makes the controller respond more slowly to real changes in the wind conditions, as the average lags behind the instantaneous value by about half the averaging time. The window size should therefore be chosen as a trade-off between smoothing out fast variations and following the slower changes in the conditions.

**Note**: in a quasi-steady simulation where the controller is updated several times for the same wind condition, for instance in the complete sail model when searching for the right angle of attack, each update is added to the average. A moving average is mainly intended for dynamic simulations, and is not necessary in quasi-steady simulations, where the flow conditions do not vary in time.
