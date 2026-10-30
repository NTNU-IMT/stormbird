# Dynamic stall

The sectional models can also be combined with a complementary model for time dependence of viscous effects. That is, the section models that build on the core [foil model](foil_model.md) can be combined with a model for so called *dynamic stall*, while the rotating cylinder can use an experimental feature to delay changes in lift with changing spin ratio (as far as the author knows, there are no established dynamic stall models for rotating cylinders, which is why this part is kept experimental for now). In classical foil theory, dynamic stall refers to the time-dependent effects that occur on the lift due to viscous effects, and which are therefore not automatically captured by for instance dynamic potential theory. These effects are complicated in practice, and the only way to model them in a simplified way is to rely on empirical equations. The model implemented in Stormbird builds on the classical approach known as the [Leishman-Beddoes model](https://www.ingentaconnect.com/content/ahs/jahs/1989/00000034/00000003/art00001) (LB model), but adapted to the stall model in the foil model. This section explains how it works, how it differs from the original LB model, and how to activate it for a lifting line simulation.

## Theory

There are two basic building blocks in the LB model that are directly relevant for the dynamic stall model in Stormbird. That is, the original formulation also contains models for the time variation in the circulatory lift and added mass, but this is handled numerically through the dynamic wake model in the lifting line method, as well as dedicated added mass models, and therefore omitted from the dynamic stall model itself in Stormbird. The first relevant block is connected to time-dependent effects on how the flow separates when it starts to move away from the trailing edge. The second block is a model for time-dependent effects due to leading edge vortex generation. The physical mechanisms are very different, but they use similar mathematical corrections.

### General about time lags

The LB model introduces dynamics to several variables (described further down) through something labeled *deficiency functions*, which is a discrete approximation of a first order lag. That is, it is a function that can introduce time lags to variables in a generic way, that is controlled through parameters that adjust the amount of lag. This section describes the basics of the general deficiency function, which is then later used throughout different steps of the complete dynamic stall model.

The time lags are defined in non-dimensional time, \\(\tau\\), measured in semi-chords travelled, where \\(V\\) is the local velocity, \\(t\\) is the time, and \\(c\\) is the local chord length:

\\[
  \tau = \frac{2 V t}{c}
\\]

In discrete time, \\(\Delta \tau = 2 V \Delta t / c\\) is the non-dimensional time step, \\(i\\) is the current time index, and \\(i-1\\) is the previous time index. The local velocity is taken as the magnitude of the velocity at the control point in each time step.

For an input signal \\(x\\) and a time constant \\(T\\), the deficiency function, \\(D\\), depends on the current and previous value of the input, the previous value of the deficiency function itself, and the time constant in the following way:

\\[
  D_{i} = D_{i-1} e^{-\Delta \tau/ T} + (x_i - x_{i-1})e^{-\Delta \tau / (2 T)}
\\]

The lagged signal is then \\(x_i - D_i\\). A time constant of zero turns the lag off, so that the lagged signal is equal to the input. The same function is used for all the lags in the model, and is available as `DynamicStallModel::deficiency_function` in the Rust library.

The plot below illustrates how the deficiency function works. The input is a pulse that starts at a given time, increases linearly to one over the ramp width, stays at one for the hold width, and then decreases linearly back to zero over the ramp width again. With a hold width of zero, the pulse is a triangle, which is similar to an angle of attack that goes quickly through stall and back again. With a ramp width of zero, the pulse is rectangular, which shows the classical step response of a first order lag. The lagged signal is computed with the discrete deficiency function, and is held constant through each time step, as in a simulation. For comparison, the plot also shows the continuous first order lag, \\(dy/d\tau = (x - y) / T\\), which the deficiency function approximates. The difference between the two shows the error from the discrete approximation, which grows with the time step relative to the time constant.

<link rel="stylesheet" href="interactive/model_explorer.css">

<div data-deficiency-function-explorer>
    <noscript>The interactive plot requires JavaScript.</noscript>
</div>

<script src="interactive/model_explorer.js"></script>
<script src="interactive/deficiency_function_explorer.js"></script>


### Time lag in the separation point and reattachment

When the flow over a foil starts to separate due to stalling, or reattaches after having stalled earlier, it may be seen as the separation point moving along the chord, causing differences in the generated lift as a result. However, this does not happen instantaneously for a given angle of attack. Viscous effects in the boundary layer can cause significant time-lag in how fast the flow is able to separate. Or in other words, if an angle of attack exceeds the stall angle for just a very short amount of time, the flow might not have time to separate. Similarly, when the angle of attack is reduced below the stall angle again, the flow needs time to reattach. The result is a hysteresis loop in the lift, where the lift is higher than the static value when the angle of attack increases through stall, and lower when it decreases.



#### The original formulation

In the original LB model, the degree of separation is described by an *effective separation point*, \\( f(\alpha) \\). It is non-dimensional, and takes the value 1 when the flow is separating cleanly on the trailing edge and goes towards zero when the separation point moves towards the leading edge. The relationship between the separation point and the force on the foil comes from Kirchhoff flow theory for a flat plate, where the separation point is forced to move. It is applied to the normal force on the foil section (i.e., not only the lift force, but a combination of lift and drag into a normal force), where \\( C_{n,static}(\alpha) \\) is the full non-linear static normal force coefficient and \\(C_{n,linear}(\alpha)\\) is the linearized version of the same coefficient:

\\[
    C_{n,static}(\alpha) = C_{n,linear}(\alpha) \left(\frac{1 + \sqrt{f(\alpha)}}{2}\right)^2
\\]

The values of \\(f(\alpha)\\) are found from static foil data, either by fitting an empirical curve, or by inverting the relationship above.

The time variation is then introduced with two first order time lags, applied in sequence. The first is a lag in the pressure distribution, applied to the attached flow normal force, with a time constant \\(T_p\\). The lagged normal force is converted to an equivalent, lagged, angle of attack, which is used to look up a separation point, \\(f'\\). The second is a lag in the boundary layer response, applied to \\(f'\\), with a time constant \\(T_f\\). This gives the dynamic separation point, \\(f''\\), which is inserted into the Kirchhoff relationship to compute the dynamic normal force.

#### Differences from the original formulation

The dynamic stall model in Stormbird keeps the core idea of the LB model, i.e., the two time lags, applied in sequence, with the same discrete deficiency functions. However, the way the degree of separation is connected to the force is adapted to the [foil model](foil_model.md). The differences, and the reasons for them, are as follows:

1. **The amount of stall replaces the Kirchhoff relationship.** The parametric foil model in Stormbird already computes the lift as a blend between a pre-stall and a post-stall lift curve, weighted by the [amount of stall](foil_model.md#amount-of-stall), \\(s(\alpha)\\). This is the same structure as in the dynamic stall model by [Hansen, Gaunaa and Madsen (2004)](https://backend.orbit.dtu.dk/ws/portalfiles/portal/7711084/ris_r_1354.pdf), which is widely used for wind turbines, where the lift is a blend between a fully attached and a fully separated lift curve. The amount of stall then plays the same role as \\(1 - f\\) in the LB model, and the time lags are applied directly to it. The reasons for this choice are:
    - **Valid for all angles of attack.** The Kirchhoff relationship can not give a force lower than \\(C_{n,linear}/4\\), even for fully separated flow. In deep stall, which is a relevant operating condition for wind propulsion devices, for instance when sailing downwind, or for suction sails with high stall angles and high lift, the static force is often lower than this. The Kirchhoff relationship can then not reproduce the static force, even in steady conditions. A blend between two lift curves does not have this limitation, and works for all angles of attack, including the full range from -180 to 180 degrees.
    - **No inversion, and therefore no edge cases.** Computing \\(f(\alpha)\\) from static data requires inverting the Kirchhoff relationship, which is only well defined for a limited range of the ratio between the static and the linear force, and which is numerically unstable close to the zero-lift angle. The amount of stall is instead given directly by the foil model.
    - **The static model is always recovered in steady conditions.** As the amount of stall comes directly from the foil model, the dynamic model gives exactly the static lift when the angle of attack is constant.
    - **Consistent with the force calculations.** The pre-stall part of the lift is used for the circulation, while the post-stall part is, by default, treated as a separate viscous lift force (see the [force calculations chapter](../line_model/force_calculations.md)). The dynamic amount of stall can therefore be used directly in the same split, so that a delayed stall gives more circulation, which is then also seen by the wake.
2. **The lift is used instead of the normal force.** This follows from the point above, as the foil model gives the lift directly. The model by Hansen, Gaunaa and Madsen uses the lift in the same way. It also means that the chordwise force from the LB model is not needed.
3. **The pressure lag is applied to the angle of attack directly.** In the LB model, the lag is applied to the attached flow normal force, which is then converted back to an angle of attack. For a linear relationship with a constant slope and zero-lift angle, this is mathematically identical to applying the lag directly to the angle of attack. The linear lift in the foil model uses \\(\sin(\alpha)\\), which can not be inverted for angles above 90 degrees, so the lag is applied to the angle of attack directly to make the model valid for all angles. 
4. **The time constants are constant.** In the original LB model, and many implementations of it, the separation time constant is modified depending on whether the foil is loading or unloading, and on the state of the leading edge vortex. In Stormbird, the time constants are constant, and do not depend on the internal state of the section model either, e.g., flap angle or suction rate. This is a simplification, but the error is believed to be small, and it keeps the number of parameters to tune low.
5. **Only the stall is lagged.** The optional [slope reduction](foil_model.md#slope-reduction) in the pre-stall lift, which can represent separation that develops gradually before the full stall, is not part of the amount of stall, and is therefore not lagged.
6. **The drag uses the same amount of stall as the lift.** The drag in the foil model is also a blend between a pre-stall and a post-stall curve, weighted by the amount of stall. The dynamic amount of stall is therefore used for the drag as well, so that the lift and the drag are always consistent. In the LB model, the drag is instead found from the dynamic normal and chordwise forces.


#### Step 1: Pressure lag

When the angle of attack changes, the pressure distribution over the foil, and in particular the suction peak near the leading edge, does not respond instantaneously. Since the separation of the boundary layer is driven by the pressure gradient, the separation responds to this lagged pressure distribution rather than to the instantaneous angle of attack. This is represented by applying a first order lag to the angle of attack, with a time constant \\(T_p\\):

\\[
  D_{p,i} = D_{p,i-1} e^{-\Delta \tau/ T_p} + (\alpha_i - \alpha_{i-1})e^{-\Delta \tau / (2 T_p)}
\\]

\\[
  \alpha_{f,i} = \alpha_i - D_{p,i}
\\]

#### Step 2: Separation lag

The static amount of stall from the foil model is then evaluated at the lagged angle of attack, rather than the instantaneous one:

\\[
  s_i' = s(\alpha_{f,i})
\\]

This gives the amount of stall that the flow would have in steady conditions, given the lagged pressure distribution. However, the boundary layer itself also needs time to respond, so a second lag is applied, with a time constant \\(T_f\\). This gives the dynamic amount of stall, \\(s''\\):

\\[
  D_{f,i} = D_{f,i-1} e^{-\Delta \tau/ T_f} + (s_i' - s_{i-1}')e^{-\Delta \tau / (2 T_f)}
\\]

\\[
  s_i'' = s_i' - D_{f,i}
\\]

The deficiency function is a discrete approximation, and with large time steps or rapidly varying input, it is not strictly guaranteed to stay within the range of the input values. The dynamic amount of stall is therefore limited to the range \\([0, 1]\\).

#### Step 3: Dynamic lift

Finally, the dynamic lift coefficient is computed from the same blend as in the foil model, but with the dynamic amount of stall. Note that the pre-stall and post-stall lift curves are evaluated at the instantaneous angle of attack, \\(\alpha\\), and not the lagged one, \\(\alpha_f\\). The lags only affect the degree of separation, and not the lift curves themselves:

\\[
  C_{L,dynamic,i} = (1 - s_i'') \\, C_{L,pre-stall}(\alpha_i) + s_i'' \\, C_{L,post-stall}(\alpha_i)
\\]

The drag coefficient is computed in the same way, from the pre-stall and post-stall drag curves in the foil model:

\\[
  C_{D,dynamic,i} = (1 - s_i'') \\, C_{D,pre-stall}(\alpha_i) + s_i'' \\, C_{D,post-stall}(\alpha_i)
\\]

In steady conditions, both deficiency functions go to zero, so that \\(\alpha_f = \alpha\\) and \\(s'' = s(\alpha)\\), and the static lift and drag coefficients from the foil model are recovered for all angles of attack.

In other words, this part of the model collapses down to two time constants, \\(T_p\\) and \\(T_f\\), that must be tuned and adjusted for a given foil profile, while the rest, how the stall depends on the angle of attack, comes directly from the foil model. Typical values reported in the literature for a NACA 0012 profile at low Mach numbers are around \\(T_p \approx 1.7\\) and \\(T_f \approx 3\\), which are used as default values.

### Leading edge vortex creation

When the angle of attack increases rapidly through the static stall angle, the flow does not necessarily only separate gradually from the trailing edge. At some point, the high suction peak near the leading edge can no longer be sustained, and the flow separates at the leading edge as well. This forms a strong vortex that is first fed by vorticity from the leading edge, and then travels downstream over the suction side of the foil. While the vortex is above the foil, its low pressure core gives an extra suction force, so that the lift becomes significantly higher than the maximum static value. When the vortex passes the trailing edge, it is shed into the wake, and the lift drops suddenly, often to values below the static post-stall lift. This is the lift overshoot, followed by an abrupt stall, that is characteristic of *deep dynamic stall*.

#### The original formulation

In the LB model, the vortex is not modelled explicitly. Instead, the extra force from the vortex is modelled as an accumulation of the force that is *lost* due to separation. The idea is that the circulation that the separated flow is no longer able to keep bound to the foil, is instead collected in the vortex, and gives a force as long as the vortex is above the foil. The input to the vortex model, often called the *vortex feed*, is the difference between the linear, attached flow, normal force and the normal force from the Kirchhoff relationship, evaluated with the dynamic separation point:

\\[
    C_{v,i} = C_{n,linear,i} \left(1 - \left(\frac{1 + \sqrt{f_i''}}{2}\right)^2 \right)
\\]

The vortex normal force, \\(C_n^v\\), is then found with the same type of deficiency function as the time lags, but with a separate time constant for the decay of the vortex, \\(T_v\\):

\\[
    C_{n,i}^v = C_{n,i-1}^v e^{-\Delta \tau / T_v} + (C_{v,i} - C_{v,i-1}) e^{-\Delta \tau / (2 T_v)}
\\]

That is, the vortex force increases when the vortex feed increases, i.e., when more of the flow separates, and decays exponentially with the time constant \\(T_v\\) otherwise. When the vortex feed is constant, the vortex force therefore goes to zero.

However, the vortex is only fed while it is above the foil. This is controlled with an onset criterion and a *vortex time*, \\(\tau_v\\), which tracks the position of the vortex:

- **Onset.** The leading edge separation is assumed to start when the lagged normal force from the pressure lag, \\(C_n'\\), exceeds a critical value, \\(C_{n1}\\). This is meant to represent a critical pressure at the leading edge. The critical value is a separate parameter, found from static foil data, typically close to the normal force at the static stall angle. As the lagged normal force is used, the onset is delayed relative to the static stall angle, as the pressure distribution needs time to build up.
- **Vortex time.** After the onset, the vortex time increases with the non-dimensional time step, \\(\tau_{v,i} = \tau_{v,i-1} + \Delta \tau\\), and is reset to zero when the onset criterion is no longer met. The vortex is assumed to travel over the chord at a constant fraction of the free stream velocity, so that it reaches the trailing edge after a fixed non-dimensional time, \\(T_{vl}\\).
- **Feeding and shedding.** The increments in the vortex feed are only added while the vortex is above the foil, i.e., \\(0 < \tau_v \leq T_{vl}\\). After the vortex has passed the trailing edge, the vortex force only decays.

The vortex force is added to the normal force from the separation model, so that the total dynamic normal force is the sum of the two.

In addition, the original model, and many implementations of it, include several further adjustments related to the vortex:

- The decay of the vortex force is accelerated after the vortex has passed the trailing edge, typically by halving \\(T_v\\).
- The separation time constant, \\(T_f\\), is modified depending on the vortex time and on whether the foil is loading or unloading.
- The center of pressure of the vortex force moves with the vortex, which gives a characteristic variation in the pitching moment.
- Some versions allow for secondary vortices, which are shed with a period of \\(T_{vl}\\) as long as the onset criterion is met.

The core of the model is therefore two additional time constants, \\(T_v\\) and \\(T_{vl}\\), and the critical normal force, \\(C_{n1}\\). Typical values reported for a NACA 0012 profile at low Mach numbers are around \\(T_v \approx 6\\) and \\(T_{vl} \approx 7\\), while for instance OpenFAST uses \\(T_{vl} = 11\\) as default.

#### Differences from the original formulation

The vortex model in Stormbird keeps the core of the LB vortex model, i.e., a vortex feed that is accumulated with a deficiency function, and a vortex time that controls when the vortex is fed. The adaptations follow the same principles as for the separation model: the force from the vortex is connected to the amount of stall rather than the Kirchhoff relationship, and as much as possible is taken directly from the foil model, so that the number of new parameters is kept low. The differences are as follows:

1. **The vortex feed is the lift lost due to the dynamic stall.** In the LB model, the vortex feed is the difference between the attached flow force and the force from the separation model. The equivalent in Stormbird is the difference between the pre-stall lift and the dynamic lift from the separation model, which, from the blend in [step 3](#step-3-dynamic-lift), simplifies to the dynamic amount of stall times the difference between the pre-stall and post-stall lift. This keeps the physical interpretation of the original model, i.e., that the lift that is lost due to stall is collected in the vortex, without the need for the Kirchhoff relationship. As for the separation model, the lift is used instead of the normal force.
2. **The onset criterion is given by the stall angle in the foil model.** In the LB model, the onset is given by a separate parameter, the critical normal force, which is typically set close to the force at the static stall angle. In Stormbird, the onset is instead defined as the point where the lagged angle of attack from the pressure lag, \\(\alpha_f\\), exceeds the mean stall angle in the foil model. This is the same as saying that the static amount of stall at the lagged angle of attack, \\(s'\\), is larger than 0.5. This is a simplification, but removes the need for a separate parameter, it is valid for both positive and negative angles of attack, with different stall angles, and it automatically follows changes in the stall angle due to the internal state of the section model, such as flap angle or suction rate. It also keeps the same physical interpretation as the original criterion, as the onset is still delayed by the pressure lag.
3. **No modifications of the time constants.** All the time constants are constant, also after the vortex has passed the trailing edge, for the same reasons as given for the separation model. This means that the vortex force decays somewhat slower after the vortex has passed the trailing edge than in the original model.
4. **No pitching moment.** The section models in Stormbird do not compute a pitching moment, so the movement of the center of pressure is not needed.
5. **No secondary vortices.** Only one vortex is created for each time the onset criterion is met. A new vortex requires that the lagged angle of attack goes below the stall angle first.
6. **Only the lift is affected.** In the LB model, the vortex force is a normal force, which therefore also contributes to the drag. In Stormbird, the vortex model only affects the lift at the moment. Note that this only applies to the vortex lift. The drag is still affected by the separation model, through the dynamic amount of stall.

#### Step 4: Vortex feed

The vortex feed is the difference between the pre-stall lift and the dynamic lift from [step 3](#step-3-dynamic-lift), which can be written as:

\\[
    C_{L,v,feed,i} = C_{L,pre-stall}(\alpha_i) - C_{L,dynamic,i} = s_i'' \left( C_{L,pre-stall}(\alpha_i) - C_{L,post-stall}(\alpha_i) \right)
\\]

#### Step 5: Vortex onset and vortex time

The vortex time, \\(\tau_v\\), is increased with the non-dimensional time step as long as the lagged angle of attack is above the mean stall angle, and reset to zero otherwise:

\\[
    \tau_{v,i} =
    \begin{cases}
        \tau_{v,i-1} + \Delta \tau & \text{if } s(\alpha_{f,i}) > 0.5 \\\\
        0 & \text{otherwise}
    \end{cases}
\\]

The vortex is above the foil, and therefore fed, when \\(0 < \tau_{v,i} \leq T_{vl}\\).

#### Step 6: Vortex lift

The vortex lift coefficient, \\(C_{L,v}\\), is found with the same deficiency function as in the other steps, with the time constant \\(T_v\\), but where the increment in the vortex feed is only included while the vortex is above the foil:

\\[
    C_{L,v,i} =
    \begin{cases}
        C_{L,v,i-1} e^{-\Delta \tau / T_v} + (C_{L,v,feed,i} - C_{L,v,feed,i-1}) e^{-\Delta \tau / (2 T_v)} & \text{if } 0 < \tau_{v,i} \leq T_{vl} \\\\
        C_{L,v,i-1} e^{-\Delta \tau / T_v} & \text{otherwise}
    \end{cases}
\\]

The total dynamic lift is then the sum of the lift from the separation model and the vortex lift:

\\[
    C_{L,total,i} = C_{L,dynamic,i} + C_{L,v,i}
\\]

The vortex lift is treated as part of the stall related lift. That is, it is handled in the same way as the post-stall lift, which by default is a separate viscous lift force, and only included in the circulation if the `include_viscous_lift_in_the_circulation` setting is used.

The vortex model also recovers the static lift in steady conditions. When the angle of attack is constant, the vortex feed is constant, so that no new vortex lift is added, and any existing vortex lift decays to zero. The vortex time also makes the model robust in deep stall, where the difference between the pre-stall and the post-stall lift can be large. Small variations in the angle of attack around a constant value well above the stall angle, for instance due to waves, will only give vortex lift during the first \\(T_{vl}\\) after the lagged angle of attack exceeded the stall angle. After that, the vortex model only gives a decaying lift, until the angle of attack goes below the stall angle again.

In other words, the vortex model adds two new time constants, \\(T_v\\) and \\(T_{vl}\\), while the onset criterion and the vortex feed come directly from the foil model. A time constant \\(T_v\\) of zero turns the vortex model off, in the same way as for the other time constants.

### Experimental: time lag in the internal state

> **Note:** This part of the model is experimental. It is mostly intended for rotor sails, and as a basis for future tests. It is turned off by default.

Most section models have an *internal state*, in addition to the angle of attack, that controls the force coefficients. The lag is applied to the internal state that is given to the section model on each line element, which is not always the same as the internal state that is set on the wing. For a rotating cylinder, the internal state of the wing is the revolutions per second, while the internal state on each line element is the [spin ratio](rotating_cylinder.md), so it is the spin ratio that is lagged. For other section models, the internal state is, for instance, a flap angle or a suction coefficient. For a single element foil, the internal state is not used, so the lag has no effect. The dynamic stall model can apply a first order time lag to the internal state, \\(x\\), with the same deficiency function as for the other lags, and a time constant \\(T_i\\):

\\[
  D_{x,i} = D_{x,i-1} e^{-\Delta \tau/ T_i} + (x_i - x_{i-1})e^{-\Delta \tau / (2 T_i)}
\\]

\\[
  x_{lagged,i} = x_i - D_{x,i}
\\]

The lag is applied before all the other steps, so that the foil model, if any, is evaluated with the lagged internal state. The lagged internal state is used for all the force coefficients, i.e., both the lift and the drag, but not for the power, which is always computed from the instantaneous internal state.

The motivation comes from rotor sails. As far as the author knows, there are no established dynamic stall models for rotating cylinders. However, the forces on a rotating cylinder depend on the boundary layer, which is driven by the rotating surface. When the spin ratio changes, for instance due to a change in the wind speed or in the rotational speed of the rotor, it can be expected that the boundary layer, and therefore the forces, need some time to adapt to the new spin ratio. The lag on the internal state is a simple way to model this. For rotating cylinders, this is the only part of the dynamic stall model that is used.

Some notes on the use of the lag:

- **It is not a model of the actuator.** The lag is applied in non-dimensional time, and is meant to represent the response of the flow to a change in the internal state. The time it takes for an actuator to change the internal state, for instance for a motor to change the rotational speed of a rotor, does not scale with the flow velocity, and should rather be modelled in the controller, for instance with the maximum change rate of the internal state of the wing.
- **The spin ratio depends on the velocity.** As the spin ratio is computed from the revolutions per second, the diameter and the local velocity on each line element, the lag also acts on changes in the velocity, for instance due to gusts or motions, even when the revolutions per second are constant.
- **The static model is recovered in steady conditions.** When the internal state is constant, the deficiency function goes to zero, and the static force coefficients are recovered.
- **The time constant is not calibrated.** There are no recommended values for \\(T_i\\) yet, which is why the default value is zero.

## Implementation

### Activating the model

The dynamic stall model is set for each wing, through the optional `dynamic_stall` field in the [wing builder](../line_model/building_line_model.md#wing-builder). The field takes a `DynamicStallModelBuilder`, which only contains the parameters of the model:

```rust
pub struct DynamicStallModelBuilder {
    pub pressure_time_constant: Option<f64>,
    pub separation_time_constant: Option<f64>,
    pub vortex_time_constant: Option<f64>,
    pub vortex_passage_time: Option<f64>,
    pub internal_state_time_constant: f64,
}
```

- `pressure_time_constant`: the non-dimensional time constant for the pressure lag, \\(T_p\\). The default value is 1.7.
- `separation_time_constant`: the non-dimensional time constant for the separation lag, \\(T_f\\). The default value is 3.0.
- `vortex_time_constant`: the non-dimensional time constant for the decay of the leading edge vortex lift, \\(T_v\\). The default value is 6.0.
- `vortex_passage_time`: the non-dimensional time it takes for the leading edge vortex to pass the trailing edge, \\(T_{vl}\\). The default value is 7.0.
- `internal_state_time_constant`: **experimental**. The non-dimensional time constant for the [lag in the internal state](#experimental-time-lag-in-the-internal-state), \\(T_i\\). The default value is 0.0, which turns the lag off.

The first four parameters control the stall in the foil model. They are optional, and values that are not set use the default values. They can only be set for section models that build on the foil model. Setting them for other section models, such as a rotating cylinder, gives an error, so that parameters are never silently ignored. The `internal_state_time_constant` can be used for all section models.

All the time constants must be zero or larger, where zero turns the corresponding lag off. A `vortex_time_constant` of zero turns the leading edge vortex off, so that only the time lags in the separation are used. The model is not used by default. To activate it with the default values, it is enough to give an empty object in the JSON input:

```json
{
    "section_points": [...],
    "chord_vectors": [...],
    "section_model": {"SingleElementFoil": {...}},
    "dynamic_stall": {}
}
```

In Python, using `stormbird_setup`, the same is done with the `DynamicStallModelBuilder` class from `stormbird_setup.section_models`:

```python
from stormbird_setup.section_models import DynamicStallModelBuilder

wing_builder = WingBuilder(
    section_points = ...,
    chord_vectors = ...,
    section_model = ...,
    dynamic_stall = DynamicStallModelBuilder(
        pressure_time_constant = 1.7,
        separation_time_constant = 3.0,
        vortex_time_constant = 6.0,
        vortex_passage_time = 7.0
    )
)
```

The full model is used together with section models that build on the foil model, i.e., `SingleElementFoil`, `MultiElementFoil` and `ActiveAirFlowFoil`. For a rotating cylinder, only the lag in the internal state is used, so the model must then be set with only the `internal_state_time_constant`:

```json
"dynamic_stall": {"internal_state_time_constant": 2.0}
```

Note that an empty object has no effect for a rotating cylinder, as the lag in the internal state is off by default.

The model depends on the time history of the flow, and is therefore only used in lifting line simulations with [dynamic simulation settings](../lifting_line/simulation_overview.md). Using it together with quasi-steady settings gives an error. The model is not yet implemented for actuator line simulations.

### Internal state and time stepping

When the line force model is built, each wing with a dynamic stall model gets a `DynamicStallModel`, which contains both the parameters and the state from the last completed time step for each line segment on the wing. That is, the internal state, the angle of attack, the deficiency functions, the amounts of stall, the vortex feed, the vortex time, and the vortex lift. For section models that do not build on the foil model, only the internal state and its deficiency function are used. The structure is defined together with the section models, as it is fundamentally a two dimensional model, while the line force model holds one optional model for each wing.

The solvers in the lifting line simulation evaluate the lift many times per time step. The dynamic stall model is therefore always evaluated based on the state from the last completed time step, and the state is not changed during the iterations. In each time step, the simulation does the following:

1. Sets the time step in the dynamic stall models, which is needed to compute the non-dimensional time step.
2. Solves for the circulation strength, where the dynamic amount of stall is used in the lift coefficients.
3. Computes the forces, with the same state as in the solver.
4. Updates the state of the dynamic stall models, based on the final velocity at the control points from the solver.

In the first time step, there is no time history available, so the model gives the static values, and the state is initialized from the result. The state is also reset when the simulation is initialized again, for instance in the `do_multiple_steps` function of the complete sail model. The steady state functions of the complete sail model reset the state before each iteration, as the iterations are not a time history, so that the results are always based on the static foil model. When the state is initialized at an angle of attack above the stall angle, any leading edge vortex is assumed to have passed the trailing edge a long time ago, so that a new vortex is only created after the lagged angle of attack has been below the stall angle first.

The dynamic stall model does not compute the forces itself. Instead, it only gives the effects that enter the force coefficients on each line segment: the lagged internal state, the dynamic amount of stall and the vortex lift. The force coefficients are then computed in two passes. First, the dynamic stall model is evaluated for each wing that has one, while wings without a dynamic stall model use the instantaneous internal state, the static amount of stall and no vortex lift. Second, the lift and drag coefficients are computed from the section model, where the internal state and the amount of stall from the first pass replace the instantaneous internal state and the static amount of stall. The lift and drag curves are always evaluated at the instantaneous angle of attack, so the lagged angle of attack is only used internally in the dynamic stall model. The power is always computed from the instantaneous internal state.

The dynamic amount of stall affects both the pre-stall lift, which is used for the circulation, and the post-stall lift, which is used either for the viscous lift force or in the circulation, depending on the `include_viscous_lift_in_the_circulation` setting. The lift from the leading edge vortex is added to the post-stall lift, and is therefore treated in the same way. See the [circulation strength chapter](../line_model/circulation_strength.md) for more.
