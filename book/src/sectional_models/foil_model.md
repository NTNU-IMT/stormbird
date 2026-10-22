# Foil model

The `Foil` structure is a *parametric model* of a single element foil section. That is, it is defined using (relatively) few parameters that are later used in a simple mathematical model to compute lift and drag for arbitrary angles of attack.

## Why a parametric model?
Other implementations of lifting line and actuator line methods often use data based models for computing the lift and drag coefficients. That is, the user must supply data on how the lift and drag varies as a function of the angle of attack, and then the solver can use this data together with interpolation or table look-up to compute force coefficients for arbitrary angles.

A data based approach is often fine, and does have some benefits. For instance, it is the only way to make a truly general model where the user has full control over the behavior of the sectional model. For this reason, there might be implementations of pure data based models in Stormbird in the future. However, the choice of using a parametric model for now was based on three reasons.

First, it becomes easier to use a parametric model as a building block for more complex foil models, where the behavior depends on some internal state, such as flap angle or suction rate. This is because the model parameters can be allowed to depend on the internal state through interpolation. See the [varying foil sub chapter](./varying_foil_model.md) for more on this.

Second, a parametric model ensures smoothness, which is beneficial when using the model together with gradient based optimization algorithms. For instance, such a method might be used to optimize the angle of attack for wing sails at a given wind direction. The smoothness is in particular practical when the expected optimal point is close to the stall angle - which it often is.

Third, tuning a parametric model is generally believed to be easier than a data based model. For instance, let's say you want to adjust the exact stall characteristics of a 2D model, to make a simplified lifting line model better fit with high-fidelity experimental data; this would not be straightforward with a data based model, but should be fairly easy if the parametric model of the lift has good parameters to control the stall behavior.

The downside of a parametric model is believed to be small, as long as the model can represent typical foil section behavior without too many simplifications. The design of the `Foil` model is intended to achieve this as best as possible[^update_to_model_note].

[^update_to_model_note]: The design of the `Foil` model is open for changes, and may be updated when a need is discovered. The goal is to find the right balance between simplicity and flexibility.

## Model overview
The model is divided in two core sub-models, labeled *pre-stall* and *post-stall*, respectively representing the behavior before and after the foil has stalled due to too high angle of attack.

For angles of attack below stall, it is assumed that both lift and drag can be represented accurately with simple functions. The lift is linear by default, but the slope can optionally be reduced above a given angle of attack. This is meant to capture separation that develops gradually before the full stall, for instance on the flap of a multi-element foil, or at low Reynolds numbers. See the [pre-stall lift](#pre-stall-lift) section below. The drag is assumed to be represented as a second order polynomial, with the minimum value at the angle where the linear part of the lift is zero. See the [pre-stall drag](#pre-stall-drag) section below. For a wing sail operating below stall, the drag will mostly be dominated by lift-induced drag forces. The viscous drag can therefore be kept simple, as the importance to the total force is relatively low.

For angles of attack above stall, both the lift and drag are assumed to be harmonic functions which primarily are adjusted by setting the *max value* after stall. This is a rough model which might not be perfect for all angles of attack, but is assumed to be close enough. A rough and approximate model for the post-stall behavior is assumed to be OK as wing sails and suction sails will most often operate below stall.

The transit between the two models is done using a [sigmoid function](https://en.wikipedia.org/wiki/Sigmoid_function), where both the transition point and the width of the transition can be adjusted. See the [stall model](#stall-model) section below.

In addition, there are factors in the model to account for added mass and lift due to the time derivative of the angle of attack. Both these effects are assumed to be linear for simplicity.

An [interactive plot](#interactive-example) at the end of this chapter can be used to explore how the parameters affect the output of the model.

## Available parameters

A view of the available fields in the `Foil` model is seen below, with further explanation of each parameter right after:

```rust
pub struct Foil {
    pub cl_zero_angle: f64,
    pub cl_initial_slope: f64,
    pub cl_slope_reduction_factor_positive: f64,
    pub cl_slope_reduction_factor_negative: f64,
    pub cl_slope_reduction_onset_angle_positive: f64,
    pub cl_slope_reduction_onset_angle_negative: f64,
    pub cl_slope_reduction_range: f64,
    pub cl_max_after_stall: f64,
    pub cd_min: f64,
    pub cd_second_order_factor: f64,
    pub cd_max_after_stall: f64,
    pub cd_power_after_stall: f64,
    pub cdi_correction_factor: f64,
    pub mean_positive_stall_angle: f64,
    pub mean_negative_stall_angle: f64,
    pub stall_range: f64,
    pub cd_bump_during_stall: f64,
    pub cd_stall_angle_offset: f64,
    pub added_mass_factor: f64,
}
```

An explanation of the parameters is given below:

- `cl_zero_angle`: Lift coefficient at zero angle of attack. This is zero by default, but can be set to a non-zero value to account for camber, flap angle or boundary layer suction/blowing.
- `cl_initial_slope`: How fast the lift coefficient increases with angle of attack, when the angle of attack is small. The default value is \\( 2 \pi \\) , which should always be the value used for a normal foil profile - with or without camber and flap - but it can also be set to a different value for instance to account for boundary layer suction/blowing.
- `cl_slope_reduction_factor_positive/negative`: Optional reduction of the lift slope for positive or negative angles of attack, as described in the [pre-stall lift](#pre-stall-lift) section below. A value of 0.0 means no reduction, while 1.0 means that the slope is reduced to zero. Is zero by default, and therefore not used.
- `cl_slope_reduction_onset_angle_positive/negative`: The angle of attack where the slope reduction is half-way developed, for positive or negative angles of attack. Both are given as positive values. The default value is 10 degrees. Only used if the corresponding reduction factor is different from zero.
- `cl_slope_reduction_range`: The range of the transition to the reduced lift slope, with the same definition as the `stall_range`. Shared by the positive and negative side. The default value is 6 degrees.
- `cl_max_after_stall`: The maximum lift coefficient after stall.
- `cd_min`: Minimum drag coefficient, which is reached at the angle where the linear part of the lift is zero.
- `cd_second_order_factor`: Factor to give the drag coefficient a second order term, as described in the [pre-stall drag](#pre-stall-drag) section below. This is zero by default.
- `cd_max_after_stall`: The maximum drag coefficient after stall.
- `cd_power_after_stall`: Power factor for the harmonic dependency of the drag coefficient after stall. Set to 1.6 by default.
- `cdi_correction_factor`: factor that can be used to correct for numerical errors in the lift-induced drag. Set to a positive value to increase the drag, and a negative value to decrease the drag. The default is zero, which means no correction.
- `mean_positive_stall_angle`: The mean stall angle for positive angles of attack, which is the mean angle where the model transitions from pre-stall to post-stall behavior. The default value is 20 degrees.
- `mean_negative_stall_angle`: The mean stall angle for negative angles of attack, which is the mean angle where the model transitions from pre-stall to post-stall behavior. The default value is 20 degrees.
- `stall_range`: The range of the stall transition, as described in the [stall model](#stall-model) section below. The default value is 6 degrees.
- `cd_bump_during_stall`: Factor to model additional drag when the foil is stalling, but that is not included in the pre- and post-stall drag models. Set to zero by default.
- `cd_stall_angle_offset`: Optional offset to the stall angle for the drag coefficient. This can be used to tune the drag curve to better fit experimental data. The default is zero, which means that stall effects on the drag starts at the same angle of attack as for the lift. When the offset is set to any value, the "amount of stall" function is shifted by this value for the case of the drag coefficient only.
- `added_mass_factor`: Factor to model added mass due to accelerating flow around the foil. Set to zero by default.

## Pre-stall lift

The pre-stall lift consists of a linear part and an optional reduction of the lift slope. The linear part is given as:

\\[
    C_{L, linear} = C_{L, 0} + a \sin(\alpha)
\\]

where \\( C_{L, 0} \\) is `cl_zero_angle` and \\( a \\) is `cl_initial_slope`. The sine of the angle of attack is used rather than the angle itself, which is practically the same for small angles, but makes the lift periodic for large angles.

### Slope reduction

Many foils have a lift curve where the slope is gradually reduced before the full stall. This can, for instance, be due to the flow separating on the flap of a multi-element foil much earlier than on the main element, or due to a slowly developing trailing edge separation at low Reynolds numbers. The result is a reduction in the lift slope, without the drop in lift that comes with the full stall.

This is modeled by letting the slope of the lift, with respect to \\( x = \sin(\alpha) \\), be reduced by sigmoid functions:

\\[
    \frac{d C_{L, pre-stall}}{dx} = a \left( 1 - r_+ \sigma_+(x) - r_- \sigma_-(-x) \right)
\\]

where \\( r_+ \\) and \\( r_- \\) are `cl_slope_reduction_factor_positive` and `cl_slope_reduction_factor_negative`. The sigmoid function is the same as the one used in the [stall model](#stall-model):

\\[
    \sigma(x) = \frac{1}{1 + e^{-k (x - x_{0})}}, \quad k = \frac{4.595}{\Delta_r}, \quad x_{0} = \sin(\alpha_{r})
\\]

where \\( \alpha_r \\) is the onset angle, given by `cl_slope_reduction_onset_angle_positive` or `cl_slope_reduction_onset_angle_negative`, and \\( \Delta_r \\) is `cl_slope_reduction_range`. With this definition, the sigmoid function is 0.01 at \\( x_0 - \Delta_r \\), 0.5 at \\( x_0 \\) and 0.99 at \\( x_0 + \Delta_r \\). That is, the onset angle is where the slope is half-way reduced.

The pre-stall lift is found by integrating the slope from zero angle of attack:

\\[
    C_{L, pre-stall} = C_{L, linear} - a \, r_+ R_+(x) + a \, r_- R_-(-x)
\\]

where \\( R(x) \\) is the integral of the sigmoid function, which can be computed exactly:

\\[
    R(x) = \int_0^x \sigma(t) \, dt = \frac{1}{k} \left[ \ln \left( 1 + e^{k (x - x_0)} \right) - \ln \left( 1 + e^{-k x_0} \right) \right]
\\]

The model has the following properties:

- Well below the onset angle, the lift is equal to the linear lift. Well above the onset angle, the slope is \\( a (1 - r) \\). A reduction factor of 1.0 therefore gives a constant lift above the onset angle.
- The lift at zero angle of attack is always equal to `cl_zero_angle`, regardless of the slope reduction.
- The reductions for the positive and negative side are both included for all angles of attack, rather than switching between them based on the sign of the angle. The positive reduction is negligible for negative angles, and vice versa, but this ensures that the lift curve is smooth also at zero angle of attack, when the positive and negative parameters are different.
- The onset angle and the range are effectively defined in terms of \\( \sin(\alpha) \\) rather than \\( \alpha \\). This makes the reduction factor exact, but means that the range is slightly different from the range in radians. The difference is small for typical angles: \\( \sin(20^\circ) = 0.342 \\), while \\( 20^\circ = 0.349 \\) radians.
- The slope reduction is not part of the linear lift, and therefore does not affect the angle where the minimum drag occurs, as described in the [pre-stall drag](#pre-stall-drag) section.

The slope reduction is meant to be used together with the stall model, which reduces the pre-stall lift further as the foil stalls. The onset angle should therefore typically be lower than the mean stall angle.

## Pre-stall drag

The pre-stall drag is modeled as a second order polynomial, where the minimum drag is assumed to occur when the lift is zero. The zero-lift angle is found from the linear part of the lift, \\( C_{L, linear} \\), as defined in the [pre-stall lift](#pre-stall-lift) section, so that the drag model automatically follows any changes in the zero-lift angle due to camber, flap angle or boundary layer suction. The pre-stall drag is given as:

\\[
    C_{D, pre-stall} = C_{D, min} + k_{D} \left( \frac{C_{L, linear}}{a} \right)^2
\\]

where \\( C_{D, min} \\) is `cd_min` and \\( k_D \\) is `cd_second_order_factor`. The lift is divided by the initial slope so that the term inside the parenthesis is approximately the angle of attack relative to the zero-lift angle, in radians. A foil without camber therefore gets a drag that varies with \\( \sin^2(\alpha) \approx \alpha^2 \\). The slope reduction in the lift is not included, as the purpose is only to find where the minimum drag occurs. If `cl_initial_slope` is zero, the zero-lift angle is undefined, and \\( \sin(\alpha) \\) is used instead.

Note that this is a simplification, as the minimum drag does not necessarily occur exactly at zero lift. However, it removes the need to tune the location of the minimum drag separately for each camber, flap angle or suction rate.

## Stall model

### Amount of stall
The transition from pre-stall to post-stall behavior is controlled by the *amount of stall*, \\( s \\), which varies smoothly from 0, meaning no stall, to 1, meaning full stall. It is computed with a sigmoid function of the absolute value of the angle of attack:

\\[
    s(\alpha) = \frac{1}{1 + e^{-k_s (|\alpha| - \alpha_s)}}, \quad k_s = \frac{4.595}{\Delta_s}
\\]

where \\( \Delta_s \\) is `stall_range` and \\( \alpha_s \\) is the mean stall angle, given by `mean_positive_stall_angle` for positive angles of attack and `mean_negative_stall_angle` for negative angles of attack. Both stall angles are given as positive values. The amount of stall is 0.01 at \\( |\alpha| = \alpha_s - \Delta_s \\), 0.5 at the mean stall angle, and 0.99 at \\( |\alpha| = \alpha_s + \Delta_s \\). The full transition therefore takes place over an interval of \\( 2 \Delta_s \\) around the mean stall angle.

Note that the stall angles are measured from zero angle of attack, and not from the zero-lift angle. For a foil with camber, flap or suction, the stall angles for positive and negative angles of attack will therefore typically be different.

### Lift
The total lift coefficient is a weighted sum of the pre-stall and post-stall lift:

\\[
    C_L = (1 - s) \, C_{L, pre-stall} + s \, C_{L, max} \sin(2 \alpha)
\\]

where \\( C_{L, pre-stall} \\) is described in the [pre-stall lift](#pre-stall-lift) section and \\( C_{L, max} \\) is `cl_max_after_stall`. The post-stall lift has its maximum value at 45 degrees, and is zero at 90 degrees, which is a typical behavior for a fully stalled foil, similar to a flat plate.

### Drag
The post-stall drag is given as:

\\[
    C_{D, post-stall} = C_{D, max} |\sin(\alpha)|^{p}
\\]

where \\( C_{D, max} \\) is `cd_max_after_stall` and \\( p \\) is `cd_power_after_stall`. The maximum drag is therefore reached at 90 degrees.

The drag uses its own amount of stall, \\( s_D \\), which can be shifted relative to the amount of stall used for the lift:

\\[
    s_D = s \left(\alpha + \delta_D \, \text{sign}(\alpha) \right)
\\]

where \\( \delta_D \\) is `cd_stall_angle_offset`. A positive offset makes the stall effects on the drag start at lower angles of attack than for the lift. The total drag coefficient is then:

\\[
    C_D = (1 - s_D) \, C_{D, pre-stall} + s_D \, C_{D, post-stall} + C_{D, bump} \frac{1 - \cos(2 \pi s_D)}{2} + k_{i} C_L^2
\\]

where \\( C_{D, pre-stall} \\) is described in the [pre-stall drag](#pre-stall-drag) section. The third term models additional drag during the stall transition, where \\( C_{D, bump} \\) is `cd_bump_during_stall`. It is zero before and after the stall, and reaches its maximum value of \\( C_{D, bump} \\) when \\( s_D = 0.5 \\). The last term is the correction of the lift-induced drag, where \\( k_i \\) is `cdi_correction_factor` and \\( C_L \\) is the total lift coefficient.

## Interactive example

The plot below shows the output of the foil model, and can be used to explore how the parameters affect the lift and drag. The parameters are adjusted with the sliders, or by typing in values, below the plot. The angles are given in degrees in the sliders, but must be given in radians in the actual input to Stormbird. The current parameters can be copied as JSON input, with angles in radians, from the bottom of the plot.

The thin gray lines show the pre-stall and post-stall models before they are combined using the amount of stall. The lowest panel shows the amount of stall for the lift, \\( s \\), and for the drag, \\( s_D \\), which are only different when `cd_stall_angle_offset` is non-zero. The parameters that are not available in the plot, such as `cdi_correction_factor` and `added_mass_factor`, are set to their default values.

<link rel="stylesheet" href="interactive/foil_model_explorer.css">

<div data-foil-model-explorer>
    <noscript>The interactive plot requires JavaScript.</noscript>
</div>

<script src="interactive/foil_model.js"></script>
<script src="interactive/foil_model_explorer.js"></script>

The plot uses a JavaScript copy of the Rust implementation of the `Foil` model, which can be found in `book/src/sectional_models/interactive/foil_model.js`.
