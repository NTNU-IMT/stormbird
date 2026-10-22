# Rotating cylinder
Model representing a rotating cylinder, where the main intended use case is to model rotor sails. The purpose is to calculate lift and drag on a two-dimensional cylinder as a function of how fast the cylinder is spinning.

The primary input variable to the model is the *spin ratio*, \\( \lambda \\), which is defined as the ratio of the velocity of the rotor surface to the wind velocity. The velocity of the rotor surface is defined as the circumference times the revolutions per second. The revolutions per second is the internal state of a wing using this model, while the spin ratio is the internal state given to the model on each line element.

Like the [foil model](foil_model.md), the rotating cylinder is a *parametric model*. That is, both the lift and the drag are computed from a few parameters, rather than interpolated from data. This gives smooth force coefficients for all spin ratios, and makes the model easy to tune to experimental or CFD data. The behavior of a rotating cylinder is, in some ways, simpler than for a foil section. The lift generally increases with increasing spin ratio, and does not stall in the same way as a foil, while the drag generally goes from a high value at zero spin ratio to a low value at normal operating spin ratios. A rotor sail is also only controlled through its rotational speed, so the lift and drag are functions of the spin ratio only.

## Available parameters

The parameters in the rotating cylinder are shown below, with further explanation in the sections that follow.

```rust
pub struct RotatingCylinder {
    pub cl_initial_slope: f64,
    pub cl_high_spin_ratio_slope: f64,
    pub cl_slope_transition_spin_ratio: f64,
    pub cl_slope_transition_range: f64,
    pub cd_start: f64,
    pub cd_end: f64,
    pub cd_angle_start: f64,
    pub cd_spin_ratio_low_drag: f64,
    pub added_mass_factor: f64,
    pub moment_of_inertia_2d: f64,
    pub cdi_correction_factor: f64,
    pub power_coefficient_polynomial: Vec<f64>,
}
```

- `cl_initial_slope`: How fast the lift coefficient increases with the spin ratio, when the spin ratio is small. The default value is 3.0.
- `cl_high_spin_ratio_slope`: How fast the lift coefficient increases with the spin ratio, when the spin ratio is well above the transition spin ratio. The default value is 1.5.
- `cl_slope_transition_spin_ratio`: The spin ratio where the transition between the two slopes is half-way developed. Given as a positive value, which is used for both positive and negative spin ratios. The default value is 3.4.
- `cl_slope_transition_range`: The range of the transition between the two slopes, with the same definition as the `stall_range` in the [foil model](foil_model.md#stall-model). The default value is 1.0.
- `cd_start`, `cd_end`, `cd_angle_start` and `cd_spin_ratio_low_drag`: Parameters for the drag, as described in the [drag](#drag) section below.
- `cdi_correction_factor`: Factor that can be used to correct for numerical errors in the lift-induced drag, in the same way as for the [foil model](foil_model.md). It adds a term to the drag coefficient that is equal to the factor times the lift coefficient squared. The default is zero, which means no correction.
- `added_mass_factor` and `moment_of_inertia_2d`: Can be used to estimate added mass forces and gyroscopic forces on the rotor. **Note**: more to come on these parameters later. They need further validation, and are therefore set to zero by default.
- `power_coefficient_polynomial`: Model for the power needed to spin the cylinder, as described in the [power coefficient](#power-coefficient) section below.

The default values for the lift and drag are based on the two-dimensional CFD results in the article "[Calculation of Flettner rotor forces using lifting line and CFD methods](https://blueoasis.pt/wp-content/uploads/2023/10/Nutts2023_proceedings_v4.pdf)". The lift parameters are fitted to the lift coefficients for spin ratios between 0 and 8 in the article, with a maximum deviation of about 0.4 in the lift coefficient. See also the [validation data section](../literature/validation_data.md) for more on this.

## Lift

The lift is modeled with an initial slope at low spin ratios, which changes smoothly to a different slope at high spin ratios. For most rotor sails, the slope at high spin ratios is lower than the initial slope, as the lift starts to level off. The model is built in the same way as the [slope reduction](foil_model.md#slope-reduction) in the foil model. The slope of the lift with respect to the spin ratio is given as:

\\[
    \frac{d C\_L}{d \lambda} = a\_0 - (a\_0 - a\_\infty) \left( \sigma(\lambda) + \sigma(-\lambda) \right)
\\]

where \\( a\_0 \\) is `cl_initial_slope` and \\( a\_\infty \\) is `cl_high_spin_ratio_slope`. The sigmoid function is the same as the one used in the [stall model](foil_model.md#stall-model) of the foil:

\\[
    \sigma(\lambda) = \frac{1}{1 + e^{-k (\lambda - \lambda\_t)}}, \quad k = \frac{4.595}{\Delta\_t}
\\]

where \\( \lambda\_t \\) is `cl_slope_transition_spin_ratio` and \\( \Delta\_t \\) is `cl_slope_transition_range`. With this definition, the sigmoid function is 0.01 at \\( \lambda\_t - \Delta\_t \\), 0.5 at \\( \lambda\_t \\) and 0.99 at \\( \lambda\_t + \Delta\_t \\). The first sigmoid function gives the transition for positive spin ratios, while the second gives the transition for negative spin ratios.

The lift is found by integrating the slope from zero spin ratio:

\\[
    C\_L = a\_0 \lambda - (a\_0 - a\_\infty) \left( R(\lambda) - R(-\lambda) \right)
\\]

where \\( R(\lambda) \\) is the integral of the sigmoid function, which can be computed exactly, as described in the [foil model](foil_model.md#slope-reduction).

The model has the following properties:

- The lift is an odd function of the spin ratio: \\( C\_L(-\lambda) = -C\_L(\lambda) \\). That is, the sign of the spin ratio only determines the direction of the lift, while the magnitude of the lift, and the spin ratio where the transition happens, is the same for both directions of rotation.
- Well below the transition spin ratio, the slope is \\( a\_0 \\). Well above, the slope is \\( a\_\infty \\). Setting the two slopes to the same value gives a lift that is linear in the spin ratio.
- The lift is smooth for all spin ratios, including at zero spin ratio, as both transitions are included for all spin ratios, rather than switching between them based on the sign of the spin ratio.
- The slope at zero spin ratio is only exactly equal to \\( a\_0 \\) when the transition spin ratio is well above the transition range, as the sigmoid functions are then practically zero at zero spin ratio. This is the case for typical rotor sail parameters.
- A transition range of zero gives a sharp change between the two slopes at the transition spin ratio.

## Drag

The drag is modeled with a simple parametric curve that goes smoothly from a high value at zero spin ratio to a low value at higher spin ratios. The curve is controlled by the following parameters:

- `cd_start`: The drag coefficient at zero spin ratio. The default value is 0.457.
- `cd_end`: The drag coefficient at larger spin ratios. The default value is 0.05.
- `cd_spin_ratio_low_drag`: The spin ratio where the drag coefficient reaches the `cd_end` value. The default value is 2.7.
- `cd_angle_start`: An angle, in radians, that controls the slope of the drag curve at zero spin ratio. A higher value gives a steeper downwards slope. The default value is 5 degrees.

The spin ratio for each line element is calculated from the internal state of the wing, which is the revolutions per second, and the local chord length and velocity. The chord length is used as the diameter of the cylinder.

## Power coefficient

The power needed to spin the cylinder is modeled with a power coefficient, as defined in the [section model overview](sectional_models_intro.md), which is a polynomial function of the absolute value of the spin ratio, \\( SR \\):

\\[
    C\_P = \sum\_i a\_i |SR|^i
\\]

The coefficients \\( a\_i \\) are given in the `power_coefficient_polynomial` field, in increasing order, starting with the constant term. The absolute value of the spin ratio is used since the power does not depend on the direction of rotation. 

The default coefficients are taken from equation (9) in the article "[Design, operation and analysis of wind-assisted cargo ships](https://doi.org/10.1016/j.oceaneng.2020.107603)", by Tillig and Ringsberg (2020):

\\[
    C\_P = 0.0234 \\\, SR - 0.0168 \\\, SR^2 + 0.0143 \\\, SR^3 - 0.0004 \\\, SR^4 + 0.0001 \\\, SR^5
\\]

The article states that the model is valid for rotors with an aspect ratio of 6 and an end disk diameter of two times the rotor diameter. In the article, the model is used for the whole rotor, together with the apparent wind velocity. In Stormbird, it is applied to each line element, with the local velocity, which is a strip theory assumption. Note also that the spin ratio is zero when the velocity is zero, so the model gives no power for a spinning rotor in still air. 

The power coefficient is used by the default [input power model](../line_model/input_power.md). Rotor sails can also use a power model that interpolates the power directly from the revolutions per second, which is independent of this coefficient.

## Interactive example

The plot below shows the output of the rotating cylinder model, and can be used to explore how the parameters affect the lift and drag. The parameters are adjusted with the sliders, or by typing in values, below the plot. The current parameters can be copied as JSON input from the bottom of the plot. The `cd_angle_start` is given in degrees in the slider, but must be given in radians in the actual input to Stormbird, which is also how it is given in the JSON output.

The thin gray lines in the lift panel show the line with the initial slope, \\( a\_0 \lambda \\), and the straight line the lift approaches well above the transition spin ratio, \\( \text{sign}(\lambda) \left( a\_\infty |\lambda| + (a\_0 - a\_\infty) \lambda\_t \right) \\). The second panel shows the slope of the lift with respect to the spin ratio. The plot is limited to spin ratios between -5 and 5, as higher values are typically not relevant for rotor sails. The parameters that are not available in the plot, such as `added_mass_factor` and `power_coefficient_polynomial`, are set to their default values.

<link rel="stylesheet" href="interactive/model_explorer.css">

<div data-rotating-cylinder-model-explorer>
    <noscript>The interactive plot requires JavaScript.</noscript>
</div>

<script src="interactive/rotating_cylinder_model.js"></script>
<script src="interactive/model_explorer.js"></script>
<script src="interactive/rotating_cylinder_model_explorer.js"></script>

The plot uses a JavaScript copy of the Rust implementation of the `RotatingCylinder` model, which can be found in `book/src/sectional_models/interactive/rotating_cylinder_model.js`.
