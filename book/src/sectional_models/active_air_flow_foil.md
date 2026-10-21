# Active air flow foil

NOTE: THIS CHAPTER IS A DRAFT. THE MODEL IS NEW, AND BOTH THE MODEL AND THE TEXT MIGHT CHANGE.

The `ActiveAirFlowFoil` structure is a model of a foil with active flow control, where the main intended use case is to model suction sails. On a suction sail, air is sucked into the foil through a porous surface on the suction side, using one or more fans. This keeps the boundary layer attached at higher angles of attack, which allows much larger lift coefficients than for a regular foil.

The model consists of two parts:

1) A [`VaryingFoil`](varying_foil_model.md) model, which gives the lift and drag as a function of the angle of attack and an internal state, such as the suction coefficient. 
2) An internal state conversion, which defines how the internal state set on the wing is converted to the internal state used by the `VaryingFoil` model on each line element.

The reason for the second part is that the most practical way to control a suction sail is often not the same as the variable that the lift and drag depend on. For instance, a suction sail is controlled by the power or the rotational speed of the fans, while the lift and drag are better described as a function of the suction coefficient. The conversion between these depends on the local flow conditions, and can therefore vary along the span of the wing.

```rust
pub struct ActiveAirFlowFoil {
    pub varying_foil: VaryingFoil,
    pub internal_state_conversion: InternalStateConversion,
}

pub enum InternalStateConversion {
    SuctionCoefficientDirect,
    PowerToPowerCoefficient,
    PowerToInternalFlow {
        internal_flow: InternalFlow,
        fan_efficiency: f64,
    },
    FanSpeedToInternalFlow {
        internal_flow: InternalFlow,
        fan: Fan,
    },
}
```

## Internal state conversions

There are four different ways to convert the internal state, which have different levels of complexity. An important point to note is that the meaning of the `internal_state_data` in the `VaryingFoil` model depends on which conversion is used.

1) `SuctionCoefficientDirect`: The internal state of the wing is the suction coefficient, which is given directly to the `VaryingFoil` model on all line elements. The foil data must therefore be given as a function of the suction coefficient. This variant is mostly useful for testing, as it does not say anything about how much power is needed. The power is therefore set to zero.
2) `PowerToPowerCoefficient`: The internal state of the wing is the power. The power is converted to a power coefficient on each line element, which is given to the `VaryingFoil` model. The foil data must therefore be given as a function of the power coefficient. This is a simplified model, which is useful when data for the lift and drag is available as a function of the power coefficient, as for the [TurboSail data](../literature/validation_data.md).
3) `PowerToInternalFlow`: The internal state of the wing is the input power to the fans. The power is converted to a suction coefficient on each line element, using a model of the internal flow in the sail and a constant fan efficiency. The foil data must therefore be given as a function of the suction coefficient. This is expected to be the most used model, and is described in more detail below.
4) `FanSpeedToInternalFlow`: The internal state of the wing is the rotational speed of the fans, in revolutions per second. The suction coefficient on each line element is found from the same internal flow model, but the fans are modeled with fan curves rather than a constant efficiency. This is the most complete model, intended for cases where data for the fans is available. It is described in the [fan model](#fan-model) section below.

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

The internal state of the wing can be both positive and negative for all conversions. The sign of the internal state is used to set the direction of the suction, i.e., which side of the foil is the suction side. The magnitude of the power, rotational speed or suction coefficient is given by the absolute value of the internal state. That is, setting a negative power does not mean that the fan *produces* power, but that the same amount of power is used to suck air in on the other side of the foil. The sign is kept in the internal state given to the `VaryingFoil` model, which should therefore contain mirrored foil data for the negative values.

### Span varying internal states
The internal state of the wing can also [vary along the span](sectional_models_intro.md#constant-and-span-varying-internal-states). For the power based conversions, a span varying internal state is interpreted as *power per span length*. For the `PowerToPowerCoefficient` conversion, the power on each line element is then the local power per span length times the length of the line element, and the power coefficient becomes:

\\[
    C_{P, i} = \frac{p'(y_i)}{0.5 \rho c_i U_i^3}
\\]

where \\( p'(y_i) \\) is the power per span length at the control point and \\( c_i \\) is the chord length. For the conversions with an internal flow model, the internal state is applied to each [internal compartment](#internal-compartments), as described below.

## The internal flow model

The `PowerToInternalFlow` and `FanSpeedToInternalFlow` conversions use a simplified model of the flow through the porous surface of the sail. The model is based on a pressure balance between the outside and the inside of the sail, and the following assumptions:

- All line elements in the same internal compartment share the same internal volume, with a uniform internal pressure. By default, the whole wing is a single compartment.
- The external pressure on the porous surface can be estimated from the lift coefficient only.
- The pressure loss through the porous surface is proportional to the velocity through the surface squared.

The input parameters to the model are shown below. The model only describes the flow through the porous surface. The fans are part of the internal state conversion.

```rust
pub struct InternalFlow {
    pub cp_zero_cl: f64,
    pub cp_gradient: f64,
    pub loss_coefficient: f64,
    pub porous_fraction: f64,
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
The fans create an internal suction, \\( s \\), which is the difference between the ambient pressure and the internal pressure in the sail. The pressure difference over the porous surface is then:

\\[
    \Delta p = C_{p, ext} \, 0.5 \rho U^2 + s
\\]

The flow through the surface is driven by this pressure difference, and is modelled using a loss coefficient, \\( K \\), given by `loss_coefficient`:

\\[
    Q = \phi A \sqrt{\frac{2 \Delta p}{\rho K}}
\\]

where \\( \phi \\) is the fraction of the area that is porous, given by `porous_fraction`. Note that only the ratio \\( \phi / \sqrt{K} \\) matters for the flow. The intention is that the porous fraction is set based on the geometry of the sail, while the loss coefficient is tuned to match experimental or CFD data.

If the internal suction is too small to overcome the external suction, the pressure difference becomes negative, which in reality means that air flows out of the sail. Since the sign of the internal state is used for the direction of the suction, such outflow is not represented in Stormbird. Line elements with outflow are instead assumed to have zero flow.

### Power with a constant fan efficiency
The power delivered to the flow is the internal suction times the total volume flow over the wing. For the `PowerToInternalFlow` conversion, the input power to the fans is found by dividing by a constant fan efficiency, \\( \eta \\), given by `fan_efficiency`:

\\[
    P = \frac{s \sum_i Q_i}{\eta}
\\]

The default value of the fan efficiency is 1.0.

## Fan model

The `FanSpeedToInternalFlow` conversion replaces the constant fan efficiency with a model of the fans, based on fan curves. The fan curves give the pressure rise over a fan, \\( \Delta p_{fan} \\), and the input power to the fan, \\( P_{fan} \\), as a function of the volume flow and the rotational speed. The model is set up with the following structures:

```rust
pub struct Fan {
    pub curves: FanCurves,
    pub number_of_fans: usize,          // Default: 1
    pub duct_loss_coefficient: f64,     // Default: 0.0
}

pub enum FanCurves {
    Dimensional {
        reference_rotational_speed: f64,
        reference_density: f64,
        data: FanCurveData,
    },
    NonDimensional {
        diameter: f64,
        data: FanCurveData,
    },
}

pub struct FanCurveData {
    pub flow_data: Vec<f64>,
    pub pressure_data: Vec<f64>,
    pub power_data: Vec<f64>,
}
```

### Fan curves
The fan curves can be given in two ways, depending on what data is available:

1) `Dimensional`: The data is given as volume flow in m³/s, pressure rise in Pa and power in W, measured at a reference rotational speed, \\( n_{ref} \\), and density, \\( \rho_{ref} \\). This is typically the format used in data sheets from fan manufacturers. 
2) `NonDimensional`: The data is given as the flow coefficient, the pressure coefficient and the power coefficient, using the diameter of the fan, \\( D \\), and the rotational speed in revolutions per second, \\( n \\):

\\[
    \phi_{fan} = \frac{Q}{n D^3}, \quad \psi_{fan} = \frac{\Delta p_{fan}}{\rho n^2 D^2}, \quad \lambda_{fan} = \frac{P_{fan}}{\rho n^3 D^5}
\\]

In both cases, the curves are scaled to other rotational speeds and densities using the fan affinity laws, which state that the flow scales with \\( n \\), the pressure rise with \\( \rho n^2 \\) and the power with \\( \rho n^3 \\). For the non-dimensional curves, this scaling is built into the definition of the coefficients. For the dimensional curves, the scaling is done relative to the reference values. 

The data must only cover the stable operating range of the fan, where the pressure rise decreases with increasing flow. Many fans have a region at low flow where this is not the case, which can give multiple possible operating points. This is checked when the model is set up, and data that does not satisfy this requirement is not accepted. Below the lowest flow in the data, the first data point is used. Above the highest flow, the curves are extrapolated linearly.

### Multiple fans and duct losses
The `number_of_fans` field specifies the number of identical fans in parallel, which share the total volume flow equally. The `duct_loss_coefficient`, \\( k \\), can be used to model the pressure loss in the ducts between the internal volume of the sail and the fans, as a function of the total volume flow:

\\[
    \Delta p_{duct} = k \left( \sum_i Q_i \right)^2
\\]

### Operating point
For a given rotational speed, the operating point of the fans is where the pressure rise over the fans balances the internal suction and the duct losses, at the total volume flow through the porous surface:

\\[
    \Delta p_{fan} \left(\sum_i Q_i(s), n \right) = s + \Delta p_{duct}
\\]

The total volume flow increases with increasing internal suction, while the pressure rise over the fans decreases with increasing flow. There is therefore only one operating point. If the fans can not overcome the external suction on any of the line elements, there is no flow. When the operating point is found, the input power is given directly by the power curve of the fans. The efficiency of the fans is therefore a result of the model, rather than an input.

## Internal compartments

Most suction sails have a single fan, and a single internal volume. However, some have several fans along the span, where the internal volume is divided into separate compartments by bulkheads. This can be modeled by dividing the wing into *internal compartments*, through the `internal_compartment_boundaries` field in the [wing builder](../line_model/building_line_model.md). The compartments are defined on the wing, rather than in the section model, as they are a property of the layout of the wing, while the section model describes the flow through the porous surface and the type of fan.

Each compartment is modeled as sealed from the others, with its own uniform internal pressure and its own fans. That is, each compartment is solved separately, in the same way as a whole wing with a single compartment. A line element belongs to the compartment that contains its control point. If a compartment boundary does not match the boundary between two line elements, the effective boundary will therefore be shifted to the closest line element boundary.

The internal state of the wing is applied to each compartment as follows:

- `PowerToInternalFlow`: A constant internal state is the total power for the wing, which is divided equally between the compartments. A span varying internal state is the power per span length, which is integrated over each compartment to give the power for the compartment.
- `FanSpeedToInternalFlow`: The rotational speed of the fans in each compartment is the mean value of the internal state over the compartment. A constant internal state therefore gives the same rotational speed for all fans. All compartments use the same fan model.

The sign of the internal state sets the direction of the suction for each compartment separately. A `PiecewiseConstant` span variation with the same boundaries as the compartments gives exactly one value per compartment, which is the most natural way to set the power or rotational speed of each fan individually.

Note that, since the internal pressure is uniform within a compartment, it is only the total power for each compartment that matters for the `PowerToInternalFlow` conversion. A variation of the power per span length *within* a compartment does not affect the result, as the distribution of the flow, and therefore the power, along the span is given by the internal flow model.

## Virtual line elements

[Virtual line elements](../line_model/building_line_model.md), for instance used to model end plates, get the same type of internal state as the rest of the wing. For instance, they get the suction coefficient that follows from the internal suction in the compartment they belong to. This keeps the circulation on the virtual line elements consistent with the rest of the wing. However, virtual line elements are not part of the sail itself, and are therefore not included in the flow through the sail or in the power.

## Solving for the suction coefficient
The internal suction is not known directly, and must be found from the internal state of the wing. This is done in two levels:

1) For a given internal suction, the suction coefficient on each line element is found. This requires a solver since the suction coefficient affects the lift coefficient through the `VaryingFoil` model, which again affects the external pressure, and therefore the suction coefficient. The solver finds the value where the two are consistent, using the [secant method](https://en.wikipedia.org/wiki/Secant_method). 
2) The internal suction is found for each compartment. For the `PowerToInternalFlow` conversion, this is the value where the power equals the power given as the internal state of the wing, found with the secant method. For the `FanSpeedToInternalFlow` conversion, this is the operating point of the fans, found with [Brent's method](https://en.wikipedia.org/wiki/Brent%27s_method). Brent's method always keeps the solution within a bracket, which is known in this case: the internal suction is always between the value where the flow into the sail starts and the maximum pressure rise of the fans.

The settings for both levels can be set through the `solver_settings` field, but the default values should normally be sufficient:

```rust
pub struct InternalFlowSolverSettings {
    pub segment_tolerance: f64,         // Default: 1e-8
    pub segment_max_iterations: usize,  // Default: 20
    pub wing_tolerance: f64,            // Default: 1e-6, relative
    pub wing_max_iterations: usize,     // Default: 50
}
```

## Power consumption and control

The power needed to operate the sail is computed by the section model for the whole wing, as described in the [section model overview](sectional_models_intro.md). For the power based conversions, this is done by integrating a power coefficient over the wing, which means that the default [input power model](../line_model/input_power.md) gives the same power as the internal state of the wing. For the `FanSpeedToInternalFlow` conversion, the efficiency of the fans depends on the operating point in each compartment. The power is therefore computed from the power curve of the fans, using the volume flow and the rotational speed in each compartment, and summed over the compartments. 

The [controller](../utils/control_system.md) can be used to set the internal state of a suction sail in two ways. Either the set points can be given directly as the wing internal state, i.e., as the power or the rotational speed of the fans, or they can be given in the units of the section model, i.e., as the suction or power coefficient. In the latter case, the set points are converted to the wing internal state based on the measured flow conditions on the wing, assuming the same suction or power coefficient along the whole span. This is an open loop estimate, so the actual suction coefficient along the wing will only approximately match the set point. With several internal compartments, the area of the wing is assumed to be divided equally between them. The controller sets a constant internal state for the wing, so internal states that vary along the span must currently be [set manually](../line_model/move_line_models.md#update-the-internal-state). 

The maximum value of the internal state can be limited through the `wing_internal_state_max_value` setting in the controller. This is the maximum power for the power based conversions, and the maximum rotational speed of the fans for the `FanSpeedToInternalFlow` conversion.

## Assumptions and limitations

The internal flow model is simplified, and the empirical parameters must be tuned to experimental or CFD data to give accurate results. Some known simplifications are listed below:

- Reynolds number effects are not modeled, neither for the external flow nor for the flow through the porous surface. The quadratic loss model is typical for perforated surfaces with relatively high through-flow velocities, while the losses through fine porous materials can be closer to linear in the velocity.
- The `PowerToInternalFlow` conversion assumes a constant fan efficiency, while it will in reality depend on the operating point of the fans. The fans are also treated as a power source that can deliver any combination of suction and volume flow, which implicitly assumes fans with variable speed. The `FanSpeedToInternalFlow` conversion removes this simplification, if fan curves are available.
- The external pressure is assumed to depend on the lift coefficient only. This is only approximately true, and mainly valid for attached flow. 
- The internal pressure is assumed to be uniform within each compartment. Losses in the ducts between the internal volume and the fans can be included in the fan model, but losses inside the internal volume itself are not modeled. Leakage between compartments is not modeled.
- Outflow through the porous surface is not modeled, as described above.

Another point to be aware of is the drag due to the momentum of the air that is sucked into the sail, often called *sink drag*. In coefficient form, it is approximately \\( 2 C_Q \\). This drag is not part of the internal flow model, but should be part of the drag data in the `VaryingFoil` model. Drag data from integrating the pressure and friction on the surface of the sail, for instance from CFD, will typically include this effect. Drag data from wake measurements will not, and must then be corrected.

## Examples
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
                "internal_flow": {
                    "cp_zero_cl": -0.5,
                    "cp_gradient": -0.8,
                    "loss_coefficient": 10.0,
                    "porous_fraction": 0.3
                },
                "fan_efficiency": 0.7
            }
        }
    }
}
```

The same model with two fans, modeled with dimensional fan curves measured at 20 revolutions per second, is shown below. Again, the values are only examples.

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
            "FanSpeedToInternalFlow": {
                "internal_flow": {
                    "cp_zero_cl": -0.5,
                    "cp_gradient": -0.8,
                    "loss_coefficient": 10.0,
                    "porous_fraction": 0.3
                },
                "fan": {
                    "curves": {
                        "Dimensional": {
                            "reference_rotational_speed": 20.0,
                            "reference_density": 1.225,
                            "data": {
                                "flow_data": [0.0, 2.0, 4.0, 6.0, 8.0],
                                "pressure_data": [600.0, 568.0, 472.0, 312.0, 88.0],
                                "power_data": [400.0, 1800.0, 2900.0, 3100.0, 1600.0]
                            }
                        }
                    },
                    "number_of_fans": 2,
                    "duct_loss_coefficient": 5.0
                }
            }
        }
    }
}
```

The two simpler conversions do not need any input, and are given as strings. For instance: `"internal_state_conversion": "PowerToPowerCoefficient"`.
