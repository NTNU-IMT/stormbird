# Active air flow foil

NOTE: THIS CHAPTER IS A DRAFT. THE MODEL IS NEW, AND BOTH THE MODEL AND THE TEXT MIGHT CHANGE.

The `ActiveAirFlowFoil` structure is a model of a foil with active flow control, where the main intended use case is to model suction sails. On a suction sail, air is sucked into the foil through a porous surface on the suction side, using a fan. This keeps the boundary layer attached at higher angles of attack, which allows much larger lift coefficients than for a regular foil.

The model consists of two parts:

1) A [`VaryingFoil`](varying_foil_model.md) model, which gives the lift and drag as a function of the angle of attack and an internal state, such as the suction coefficient. 
2) An internal state conversion, which defines how the internal state set on the wing is converted to the internal state used by the `VaryingFoil` model on each line element.

The reason for the second part is that the most practical way to control a suction sail is often not the same as the variable that the lift and drag depend on. For instance, a suction sail is controlled by how much power is given to the fan, while the lift and drag are better described as a function of the suction coefficient. The conversion between these two depends on the local flow conditions, and can therefore vary along the span of the wing.

```rust
pub struct ActiveAirFlowFoil {
    pub varying_foil: VaryingFoil,
    pub internal_state_conversion: InternalStateConversion,
}

pub enum InternalStateConversion {
    SuctionCoefficientDirect,
    PowerToPowerCoefficient,
    PowerToInternalFlow(InternalFlow),
}
```

## Internal state conversions

There are three different ways to convert the internal state, which have different levels of complexity. An important point to note is that the meaning of the `internal_state_data` in the `VaryingFoil` model depends on which conversion is used.

1) `SuctionCoefficientDirect`: The internal state of the wing is the suction coefficient, which is given directly to the `VaryingFoil` model on all line elements. The foil data must therefore be given as a function of the suction coefficient. This variant is mostly useful for testing, as it does not say anything about how much power is needed. The power is therefore set to zero.
2) `PowerToPowerCoefficient`: The internal state of the wing is the power. The power is converted to a power coefficient on each line element, which is given to the `VaryingFoil` model. The foil data must therefore be given as a function of the power coefficient. This is a simplified model, which is useful when data for the lift and drag is available as a function of the power coefficient, as for the [TurboSail data](../literature/validation_data.md).
3) `PowerToInternalFlow`: The internal state of the wing is the input power to the fan. The power is converted to a suction coefficient on each line element, using a model of the internal flow in the sail. The foil data must therefore be given as a function of the suction coefficient. This is the most complete model, and is described in more detail below.

The suction coefficient, \\( C_Q \\), and the power coefficient, \\( C_P \\), are defined as:

\\[
    C_Q = \frac{Q}{U A}, \quad C_P = \frac{P}{0.5 \rho A U^3}
\\]

where \\( Q \\) is the volume flow through the porous surface, \\( P \\) is the power, \\( \rho \\) is the density, \\( A \\) is the area and \\( U \\) is the velocity. In Stormbird, the area and velocity are always the local values on each line element, where the area is the chord length times the length of the line element. It is important that the data for the `VaryingFoil` model uses the same definitions. 

For the `PowerToPowerCoefficient` conversion, the power is distributed over the line elements proportional to their area. The power coefficient on each line element then becomes:

\\[
    C_{P, i} = \frac{P}{0.5 \rho A_{wing} U_i^3}
\\]

where \\( A_{wing} \\) is the total area of the wing and \\( U_i \\) is the local velocity.

### The direction of the suction

The internal state of the wing can be both positive and negative for all conversions. The sign of the internal state is used to set the direction of the suction, i.e., which side of the foil is the suction side. The magnitude of the power or suction coefficient is given by the absolute value of the internal state. That is, setting a negative power does not mean that the fan *produces* power, but that the same amount of power is used to suck air in on the other side of the foil. The sign is kept in the internal state given to the `VaryingFoil` model, which should therefore contain mirrored foil data for the negative values.

## The internal flow model

The `PowerToInternalFlow` conversion uses a simplified model of the flow through the porous surface of the sail. The model is based on a pressure balance between the outside and the inside of the sail, and the following assumptions:

- All line elements on a wing share the same internal volume, with a uniform internal pressure.
- The external pressure on the porous surface can be estimated from the lift coefficient only.
- The pressure loss through the porous surface is proportional to the velocity through the surface squared.

The input parameters to the model are shown below:

```rust
pub struct InternalFlow {
    pub cp_zero_cl: f64,
    pub cp_gradient: f64,
    pub loss_coefficient: f64,
    pub porous_fraction: f64,
    pub fan_efficiency: f64,
    pub solver_settings: InternalFlowSolverSettings,
}
```

### External pressure
The external pressure coefficient on the porous surface is modeled as a linear function of the lift coefficient:

\\[
    C_{p, ext} = C_{p, 0} + \frac{d C_p}{d C_L} C_L
\\]

where \\( C_{p, 0} \\) is given by `cp_zero_cl` and \\( d C_p / d C_L \\) is given by `cp_gradient`. Since the porous surface is on the suction side, both values will typically be negative. That is, the external pressure on the porous surface gets lower when the lift increases. The lift coefficient is defined as positive when the lift is directed towards the suction side. 

### Flow through the porous surface
The fan creates an internal suction, \\( s \\), which is the difference between the ambient pressure and the internal pressure in the sail. The pressure difference over the porous surface is then:

\\[
    \Delta p = C_{p, ext} \, 0.5 \rho U^2 + s
\\]

The flow through the surface is driven by this pressure difference, and is modelled using a loss coefficient, \\( K \\), given by `loss_coefficient`:

\\[
    Q = \phi A \sqrt{\frac{2 \Delta p}{\rho K}}
\\]

where \\( \phi \\) is the fraction of the area that is porous, given by `porous_fraction`. Note that only the ratio \\( \phi / \sqrt{K} \\) matters for the flow. The intention is that the porous fraction is set based on the geometry of the sail, while the loss coefficient is tuned to match experimental or CFD data.

If the internal suction is too small to overcome the external suction, the pressure difference becomes negative, which in reality means that air flows out of the sail. Since the sign of the internal state is used for the direction of the suction, such outflow is not represented in Stormbird. Line elements with outflow are instead assumed to have zero flow.

### Power
The power delivered to the flow is the internal suction times the total volume flow over the wing. The input power to the fan is found by dividing by the fan efficiency, \\( \eta \\), given by `fan_efficiency`:

\\[
    P = \frac{s \sum_i Q_i}{\eta}
\\]

The fan efficiency is assumed to be constant, with a default value of 1.0.

### Solving for the suction coefficient
The internal suction is not known directly, and must be found from the power given as the internal state of the wing. This is done in two levels, using the [secant method](https://en.wikipedia.org/wiki/Secant_method):

1) For a given internal suction, the suction coefficient on each line element is found. This requires a solver since the suction coefficient affects the lift coefficient through the `VaryingFoil` model, which again affects the external pressure, and therefore the suction coefficient. The solver finds the value where the two are consistent. 
2) The internal suction is found by solving for the value where the power, as computed above, equals the power given as the internal state of the wing.

The settings for both solvers can be set through the `solver_settings` field, but the default values should normally be sufficient:

```rust
pub struct InternalFlowSolverSettings {
    pub segment_tolerance: f64,       // Default: 1e-8
    pub segment_max_iterations: usize, // Default: 20
    pub power_tolerance: f64,         // Default: 1e-6, relative to the power
    pub power_max_iterations: usize,   // Default: 20
}
```

## Power consumption and control

The power needed to operate the sail is computed through the power coefficient of the section model, as described in the [section model overview](sectional_models_intro.md). This means that the default [input power model](../line_model/input_power.md) will give the same power as the internal state of the wing for both of the power based conversions. 

The [controller](../utils/control_system.md) can be used to set the internal state of a suction sail in two ways. Either the set points can be given directly as the wing internal state, i.e., as the power, or they can be given in the units of the section model, i.e., as the suction or power coefficient. In the latter case, the set points are converted to power based on the measured flow conditions on the wing, assuming the same suction or power coefficient along the whole span. This is an open loop estimate, so the actual suction coefficient along the wing will only approximately match the set point. 

The maximum power can be limited through the `wing_internal_state_max_value` setting in the controller, since the internal state of the wing is the power for both of the power based conversions. 

## Assumptions and limitations

The internal flow model is simplified, and the empirical parameters must be tuned to experimental or CFD data to give accurate results. Some known simplifications are listed below:

- Reynolds number effects are not modeled, neither for the external flow nor for the flow through the porous surface. The quadratic loss model is typical for perforated surfaces with relatively high through-flow velocities, while the losses through fine porous materials can be closer to linear in the velocity.
- The fan efficiency is assumed to be constant, while it will in reality depend on the operating point of the fan. The fan is also treated as a power source that can deliver any combination of suction and volume flow, which implicitly assumes a fan with variable speed.
- The external pressure is assumed to depend on the lift coefficient only. This is only approximately true, and mainly valid for attached flow. 
- The internal pressure is assumed to be uniform along the span, which means that losses in the internal ducting are not modeled.
- Outflow through the porous surface is not modeled, as described above.

Another point to be aware of is the drag due to the momentum of the air that is sucked into the sail, often called *sink drag*. In coefficient form, it is approximately \\( 2 C_Q \\). This drag is not part of the internal flow model, but should be part of the drag data in the `VaryingFoil` model. Drag data from integrating the pressure and friction on the surface of the sail, for instance from CFD, will typically include this effect. Drag data from wake measurements will not, and must then be corrected.

## Example
Below is an example of the input for a section model using the `PowerToInternalFlow` conversion, given as JSON. The foil data is simplified to only include three internal states, with mirrored data for the negative suction coefficient. The values for the internal flow model are only examples, and are not tuned to any data.

```json
{
    "ActiveAirFlowFoil": {
        "varying_foil": {
            "internal_state_data": [-0.05, 0.0, 0.05],
            "foils_data": [
                { "cl_zero_angle": -1.0 },
                { "cl_zero_angle": 0.0 },
                { "cl_zero_angle": 1.0 }
            ]
        },
        "internal_state_conversion": {
            "PowerToInternalFlow": {
                "cp_zero_cl": -0.5,
                "cp_gradient": -0.8,
                "loss_coefficient": 10.0,
                "porous_fraction": 0.3,
                "fan_efficiency": 0.7
            }
        }
    }
}
```

The two simpler conversions do not need any input, and are given as strings. For instance: `"internal_state_conversion": "PowerToPowerCoefficient"`.
