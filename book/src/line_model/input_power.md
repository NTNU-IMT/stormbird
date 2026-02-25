# Input power

Some sail types, like rotor sail and suction sails require input power to operate. This is modeled entirely empirically in Stormbird. The structure that controls these models are shown below.

MORE TO COME ON THIS LATER

```rust
pub struct InputPowerData {
    pub section_models_internal_state_data: Vec<Float>,
    pub input_power_coefficient_data: Vec<Float>,
}

pub enum InputPowerModel {
    NoPower,
    InternalStateAsPowerCoefficient,
    InterpolatePowerCoefficientFromInternalState(InputPowerData),
    InterpolateFromInternalStateOnly(InputPowerData),
}
```
