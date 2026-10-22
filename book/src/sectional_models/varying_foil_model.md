# Varying foil model

The `VaryingFoil` structure is a model of a foil where the output can be allowed to depend on some internal state. The typical example is a flap angle, for a two-element foil, or suction rate, for a suction sail. To achieve this, the varying foil model uses multiple `Foil` models, each belonging to a specific internal value. Linear interpolation is used for all `Foil` model parameters for each state between the input data.

The `VaryingFoil` model is used by two of the variants in the [section model enum](sectional_models_intro.md): the `MultiElementFoil` variant uses it directly, while the `ActiveAirFlowFoil` variant uses it together with a conversion of the internal state. See the [active air flow foil chapter](active_air_flow_foil.md) for more on the latter.

## How to set up a VaryingFoil structure

As a general case, you need to tune several `Foil` models for multiple values of whatever you want to use as an internal state of the wing. If this is a flap angle, you would need data for the lift and drag coefficients as a function of angles of attack for multiple discrete flap angles. When a unique model is generated for each value of the flap angle, this can be given as input to the `VaryingFoil` structure, along with the flap angle data as `internal_state_data`. See the example at the end of this chapter.

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

The `internal_state_data` and `foils_data` must have the same length, where each `Foil` model belongs to the internal state value with the same index. The `internal_state_data` must be strictly increasing, and contain at least one value. This is checked when the model is set up.

For an internal state between two values in the data, all the parameters of the `Foil` model are linearly interpolated between the two neighboring foils. Outside the range of the data, the first or last foil is used, without extrapolation.

## Positive and negative internal states

The internal state can also be negative. This is, for instance, used to switch the direction of a flap or the suction side of a suction sail. There are two ways to give the data for negative internal states:

1) **One-sided data**: If the `internal_state_data` only contains values with one sign, e.g., only positive values, the foil is assumed to be symmetric with respect to the sign of the internal state. An internal state with the opposite sign then gives the *mirrored* foil at the absolute value of the internal state. That is, the data only has to be given for one direction of the flap or suction. Zero counts as positive, so data starting at zero is also one-sided.
2) **Two-sided data**: If the `internal_state_data` contains both positive and negative values, the data is used directly for all internal states, without any mirroring. This can be used if the foil is not symmetric, for instance, for a cambered main element with a flap that can be deflected to both sides.

The mirrored foil is the same foil profile with the suction side and the pressure side swapped, which means that the lift and drag satisfy:

\\[
    C\_{L, mirrored}(\alpha) = -C\_L(-\alpha), \quad C\_{D, mirrored}(\alpha) = C\_D(-\alpha)
\\]

For the [foil model](foil_model.md), this is achieved exactly by changing the sign of `cl_zero_angle`, and swapping all parameters that are given separately for positive and negative angles of attack, i.e., the stall angles and the parameters for the lift slope reduction. The other parameters are already symmetric with respect to the angle of attack, and are not changed.

There are no requirements on which values the one-sided data must contain. For instance, it is possible to give data for a single flap angle only, which is then used for all positive internal states, while the mirrored foil is used for all negative internal states. This is practical when the sail is only operated with a fixed flap angle to either side. If the behavior for small internal states is important, the data should include an internal state of zero, with a foil that is symmetric, i.e., with `cl_zero_angle` equal to zero and the same parameters for positive and negative angles of attack. Otherwise, the foil changes abruptly when the internal state changes sign.

An example of one-sided data for a two-element wing sail, where the flap angle is given in radians, is shown below. A negative flap angle of, for instance, -15 degrees gives the mirror image of the foil at 15 degrees, with a `cl_zero_angle` of -1.0 and a `mean_negative_stall_angle` of 17.5 degrees.

```json
{
    "MultiElementFoil": {
        "internal_state_data": [0.0, 0.5236],
        "foils_data": [
            { "cl_zero_angle": 0.0, "cd_min": 0.01 },
            { "cl_zero_angle": 2.0, "cd_min": 0.01, "mean_positive_stall_angle": 0.2618 }
        ]
    }
}
```