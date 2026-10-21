# Input power

Some sail types, like rotor sails and suction sails, require input power to operate. This is modeled empirically in Stormbird, with a model that is specified for each wing. The structures that control these models are shown below.

```rust
pub struct InputPowerData {
    pub internal_state_data: Vec<Float>,
    pub input_power_data: Vec<Float>,
}

pub enum InputPowerModel {
    FromSectionModelPowerCoefficient,
    NoPower,
    DirectFromInternalState,
    InterpolateFromInternalState(InputPowerData),
}
```

The different variants work as follows:

1) `FromSectionModelPowerCoefficient`: The power is calculated by the [section model](../sectional_models/sectional_models_intro.md). For most section models, this is the power coefficient on each line element, integrated over the wing: \\( P = \sum_i C_{P, i} \, 0.5 \rho A_i U_i^3 \\). Section models that do not need power, such as regular foils, have a power coefficient of zero. Some section models compute the power for the whole wing directly, such as an active air flow foil with fan curves. This is the default, and is the recommended choice in most cases, as it keeps the power model together with the rest of the section model.
2) `NoPower`: The wing does not need power at all. This gives the same result as the default for section models without power, but can be used to explicitly turn off the power calculation for a wing.
3) `DirectFromInternalState`: The absolute value of the internal state of the wing is used directly as the power. This is a simple solution for, for instance, suction sails where the internal state is the power. If the internal state varies along the span, it is interpreted as power per span length, and the absolute value is integrated over the span.
4) `InterpolateFromInternalState`: The power is interpolated from the data in `InputPowerData`, as a function of the absolute value of the internal state of the wing. This can, for instance, be used to model the power of a rotor sail directly as a function of the revolutions per second. If the internal state varies along the span, the mean of the absolute value over the span is used.

For the [active air flow foil](../sectional_models/active_air_flow_foil.md) with a power based internal state conversion, the default model will give the same power as the `DirectFromInternalState` variant, as the power coefficient of the section model is computed to be consistent with the power given as the internal state. When the fans are modeled with fan curves, the internal state is the rotational speed of the fans, and the default model gives the power from the fan curves.
