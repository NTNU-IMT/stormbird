# Varying foil model

The `VaryingFoil` structure is a model of a foil where the output can be allowed to depend on some internal state. The typical example is a flap angle, for a two-element foil, or suction rate, for a suction sail. To achieve this, the varying foil model uses multiple `Foil` models, each belonging to a specific internal value. Linear interpolation is used for all `Foil` model parameters for each state between the input data.

The `VaryingFoil` model is used by two of the variants in the [section model enum](sectional_models_intro.md): the `MultiElementFoil` variant uses it directly, while the `ActiveAirFlowFoil` variant uses it together with a conversion of the internal state. See the [active air flow foil chapter](active_air_flow_foil.md) for more on the latter.

## How to set up a VaryingFoil structure

As a general case, you need to tune several `Foil` models for multiple values of whatever you want to use as an internal state of the wing. If this is a flap angle, you would need data for the lift and drag coefficients as a function of angles of attack for multiple discrete flap angles. When a unique model is generated for each value of the flap angle, this can be given as input to the `VaryingFoil` structure, along with the flap angle data as `internal_state_data`. An example will come...

## What if I want to model a three-element foil?
More complex models with more internal variables might be added in the future. This will be done when the use case presents itself, and likely not before. However, it is still possible to use the `VaryingFoil` structure to model sails with more control parameters in a slightly simplified way.

For instance, let's assume you want to model a three-element foil, where there is both a flap and a leading edge slot. Currently it is not possible to model changing values of the slot- and flap-angle independently directly in Stormbird. However, if you assume some relationship between the flap angle and the slot angle, the `VaryingFoil` structure can still be used. 

This might not be such a large simplification for practical use cases[^more_investigation_note]. The point of a multi-element foil is both to create larger maximum lift forces and to reduce the drag force for a given lift force. In a lifting line model - and also mostly for wings in general - the lift-induced velocities are not very affected by *how* the lift is created. Rather, it is just the value of the lift-coefficient that matters. For a given *wanted lift coefficient* it seems reasonable that there is always a single optimum combination of flap- and slot-angle, which *probably* can be computed independently of three-dimensional effects. As such, reducing the model to a single internal variable *might* not be a big problem. 

[^more_investigation_note]: This is currently a hypothesis which we hope to be able to explore more in the future...

## Available parameters

The available parameters in the structure are shown below. 

```rust
pub struct VaryingFoil {
    pub internal_state_data: Vec<f64>,
    pub foils_data: Vec<Foil>,
}
```

The `internal_state_data` and `foils_data` must have the same length, where each `Foil` model belongs to the internal state value with the same index. 

The internal state can also be negative. This is, for instance, used to switch the direction of a flap or the suction side of a suction sail, by giving mirrored `Foil` models for the negative internal state values. 