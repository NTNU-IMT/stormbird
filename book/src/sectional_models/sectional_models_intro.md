# Section models

The primary purpose of the sectional models is to compute non-dimensional lift and drag as a function of the local flow velocity at each line element. That is, they are two-dimensional models which are used together with the chord vector and local flow velocity to compute the total force on each line section. In both lifting line and actuator line simulations, three-dimensional effects are modeled by altering the effective velocity experienced by these two-dimensional models. 

Different sectional models are necessary for different sail types. This is handled by implementing the sectional model as an enum, with a definition as shown below:

```rust
pub enum SectionModel {
    SingleElementFoil(Foil),
    MultiElementFoil(VaryingFoil),
    ActiveAirFlowFoil(ActiveAirFlowFoil),
    RotatingCylinder(RotatingCylinder),
    EffectiveWindSensor,
}
```

The variants use four different underlying models, which each have their own sub-chapter for more details. A short overview is given below:

1) `SingleElementFoil` uses the [`Foil`](foil_model.md) model, which represents a single element foil profile. It is suitable for modelling single element wing sails.
2) `MultiElementFoil` uses the [`VaryingFoil`](varying_foil_model.md) model, which extends the `Foil` model to allow the output to be dependent on an internal state. The internal state is typically a flap angle, for a two-element foil, or a combination of multiple element configurations, for instance to model a three-element foil.
3) `ActiveAirFlowFoil` uses the [`ActiveAirFlowFoil`](active_air_flow_foil.md) model, which is intended for foils with active flow control, such as suction sails. It also uses a `VaryingFoil` model for the lift and drag, but adds a conversion between the internal state set on the wing, such as the fan power, and the internal state used by the foil model, such as the suction coefficient.
4) `RotatingCylinder` uses the [`RotatingCylinder`](rotating_cylinder.md) model, which represents a cylinder where the rotational speed can be varied to alter the force output. This is intended to be used to model rotor sails.
5) `EffectiveWindSensor` is a special model without any input parameters that always gives zero lift and drag. A wing with this model therefore does not produce any forces, but can be used to measure the effective wind velocity at its control points.

More sectional models can be added in the future. The only requirement is that each model must be able to compute the necessary force coefficients. However, the goal is to cover most use cases with the current models. 

## Internal states

All wings in a line force model have an *internal state*, which can be set for each wing, for instance by a [controller](../utils/control_system.md). What the internal state means depends on the section model:

- `SingleElementFoil` and `EffectiveWindSensor`: the internal state is not used.
- `MultiElementFoil`: the internal state is passed directly to the `VaryingFoil` model, and is typically the flap angle.
- `ActiveAirFlowFoil`: the internal state is either the power, the rotational speed of the fans, or the suction coefficient, depending on the settings of the model. It is converted to the internal state of the `VaryingFoil` model on each line element. See the [active air flow foil chapter](active_air_flow_foil.md) for more.
- `RotatingCylinder`: the internal state is the revolutions per second of the cylinder, which is converted to a spin ratio on each line element, using the local chord length (i.e., the diameter) and the local velocity.

This means that the internal state given to the section model on each line element can be different from the internal state set on the wing. The conversion is managed by the line force model structure, and is not something the user needs to think about when running a simulation. However, it is important to know which units the internal state is given in when setting up the models and the controller.

### Constant and span varying internal states
In most cases, the internal state is a single number for each wing, which is the same along the whole span. However, it is also possible to give an internal state that varies along the span, for instance to model a sail with several fans along the span, or a flap angle that varies along the span. The internal state of a wing is therefore represented by the following enums:

```rust
pub enum WingInternalState {
    Constant(f64),
    SpanVarying(SpanVariation),
}

pub enum SpanVariation {
    Linear { locations: Vec<f64>, values: Vec<f64> },
    PiecewiseConstant { boundaries: Vec<f64>, values: Vec<f64> },
}
```

The locations along the span are given as non-dimensional span positions, running from -0.5 at one end of the wing to 0.5 at the other. The span variation does not have to match the resolution of the line elements. There are two ways to give the variation:

1) `Linear`: The value is linearly interpolated between the given locations, and is constant outside of them.
2) `PiecewiseConstant`: The value is constant within zones along the span, defined by the boundaries between them. There must be one more boundary than values.

For each line element, the value of the span variation at the control point is used. The exception is section models where the internal state applies to a part of the wing rather than to a single line element, such as an [active air flow foil](active_air_flow_foil.md) with several internal compartments. The value is then averaged or integrated over each part of the wing, which is done exactly, without depending on the resolution of the line elements.

When a single number is needed to represent a span varying internal state, for instance as the current state in the controller, the mean value over the span is used. See the chapter on how to [move and modify a line model](../line_model/move_line_models.md#update-the-internal-state) for how to set the internal state, both as constant values and as span variations.

Since the models are handled through an enum, they may also require different inputs in their own functions for calculating lift and drag. For instance, the foil models require the angle of attack as input, while the `RotatingCylinder` only uses the spin ratio. Again, the right input is managed by the line force model structure.

## Power coefficient

In addition to lift and drag, each section model can compute a *power coefficient*, which is used to estimate the power needed to operate the sail. The power coefficient is defined as:

\\[
    C\_P = \frac{P}{0.5 \rho A U^3}
\\]

where \\( P \\) is the power, \\( \rho \\) is the density, \\( A \\) is the area of the line element and \\( U \\) is the local velocity. Models that do not need power, such as regular foils, return zero. 

The power for a whole wing is computed by the section model, and is used by the default [input power model](../line_model/input_power.md). For most section models, this is simply the power coefficient integrated over the wing:

\\[
    P\_{wing} = \sum\_i C\_{P, i} \\, 0.5 \rho A\_i U\_i^3
\\]

However, some models need information about the whole wing to compute the power. For instance, for an [active air flow foil](active_air_flow_foil.md) where the fans are modeled with fan curves, the efficiency of the fans depends on the total volume flow through the wing. The power is then computed directly for the whole wing, rather than through the power coefficient on each line element.

To get an overview of how to set up the different variants, see the sub-chapters for each model.
