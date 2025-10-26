# Control system

NOTE: the control system functionality is planned to be changed in version 0.8.0. Right now, the same control logic must be applied for all sails in the simulation, which is an necessary limitation. This will therefore be changed soon! More detailed documentation for the control system logic will therefore come later.

The library contain different control system that can be used to automatically adjust the sail settings for different wind conditions. The goal is to have a system that covers most uses cases. The controller can automatically be activated for different versions of the Stormbird library. However, it is also usually possible to run custom control systems in various ways with the different APIs.

The structures used to set up a control system is shown below:

```rust
pub struct ControllerBuilder {
    pub logic: ControllerLogic,
    pub flow_measurement_settings: FlowMeasurementSettings,
    pub time_steps_between_updates: usize,
    pub start_time: Float,
    pub max_local_wing_angle_change_rate: Option<f64>,
    pub max_internal_section_state_change_rate: Option<f64>,
    pub moving_average_window_size: Option<usize>,
    pub use_input_velocity_for_apparent_wind_direction: bool,
}

pub struct ControllerLogic {
    pub apparent_wind_directions_data: Vec<f64>,
    pub angle_of_attack_set_points_data: Option<Vec<f64>>,
    pub section_model_internal_state_set_points_data: Option<Vec<f64>>,
    pub internal_state_type: InternalStateType,
    pub use_effective_angle_of_attack: bool,
}

pub enum InternalStateType {
    Generic,
    SpinRatio(SpinRatioConversion),
}

pub struct SpinRatioConversion {
    diameter: f64,
    max_rps: f64,
}

pub struct FlowMeasurementSettings {
    pub angle_of_attack: MeasurementSettings,
    pub wind_direction: MeasurementSettings,
    pub wind_velocity: MeasurementSettings,
}

pub struct MeasurementSettings {
    pub measurement_type: MeasurementType,
    pub start_index: usize,
    pub end_offset: usize,
}
```
