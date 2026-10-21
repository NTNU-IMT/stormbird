# Rotating cylinder
Model representing a rotating cylinder, where the main intended use case is to model rotor sails. The purpose is to calculate lift and drag on a two-dimensional cylinder as a function of how fast the cylinder is spinning.

The primary input variable to the model is the *spin ratio*, which is defined as the ratio of the velocity of the rotor to the wind velocity. The velocity of the rotor is defined as the circumference times the rotations per second. The rotations per second is the internal state of a wing using this model, while the spin ratio is the internal state given to the model on each line element.

Unlike the [foil model](foil_model.md), the lift in the rotating cylinder model is data based and not parametric. There are primarily two reasons for this: 

1) The behavior of lift and drag on a spinning cylinder is, in some ways, simpler than a foil section. The lift will generally increase with increasing spin-ratios, and not experience stall in the same way as a foil. In addition, the two-dimensional drag is generally going from a high value at zero spin-ratio to a very low value at normal operating spin-ratios. A few data points are therefore often enough to capture the behavior of a rotating cylinder.
2) It is not common, at least not yet, to combine a rotating cylinder with other control mechanisms. For instance, for wing sails and suction sails it is common to control the sails with both a flap angle or suction rate, and the angle of attack. A rotor sail is only controlled through its rotational speed. A simple one dimensional data model is therefore assumed to be sufficient for rotor sails (lift and drag as function of spin ratio only).

## Available parameters

The parameters in the rotating cylinder are listed below.

The `cl_data` and `spin_ratio_data` are the most important parts of the model. They specify how the lift is dependent on the spin ratio. All variables have default values based on the results in the article "[Calculation of Flettner rotor forces using lifting line and CFD methods](https://blueoasis.pt/wp-content/uploads/2023/10/Nutts2023_proceedings_v4.pdf)". See also the [validation data section](../literature/validation_data.md) for more on this.

The drag is modeled with a simple parametric curve that goes smoothly from a high value at zero spin ratio to a low value at higher spin ratios. The curve is controlled by the following parameters:

- `cd_start`: The drag coefficient at zero spin ratio. The default value is 0.457.
- `cd_end`: The drag coefficient at larger spin ratios. The default value is 0.05.
- `cd_spin_ratio_low_drag`: The spin ratio where the drag coefficient reaches the `cd_end` value. The default value is 2.7.
- `cd_angle_start`: An angle, in radians, that controls the slope of the drag curve at zero spin ratio. A higher value gives a steeper downwards slope. The default value is 5 degrees.

The `cdi_correction_factor` can be used to correct for numerical errors in the lift-induced drag, in the same way as for the [foil model](foil_model.md). It adds a term to the drag coefficient that is equal to the factor times the lift coefficient squared. The default is zero, which means no correction.

The `added_mass_factor` and `moment_of_inertia_2d` parameters can be used to estimate added mass forces and gyroscopic forces on the rotor. **Note**: more to come on these parameters later. They need further validation, and are therefore set to zero by default.

The `power_coefficient_polynomial` specifies a model for the power needed to spin the cylinder, as described in the section below.

The spin ratio for each section is calculated based on the internal state of the wing, which is the revolutions per second, and the local chord length and velocity. Then, the lift in 2D is interpolated from the input data values, while the drag in 2D is calculated from the parametric drag curve. 

```rust
pub struct RotatingCylinder {
    pub spin_ratio_data: Vec<f64>,
    pub cl_data: Vec<f64>,
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

## Power coefficient

The power needed to spin the cylinder is modeled with a power coefficient, as defined in the [section model overview](sectional_models_intro.md), which is a polynomial function of the absolute value of the spin ratio, \\( SR \\):

\\[
    C_P = \sum_i a_i |SR|^i
\\]

The coefficients \\( a_i \\) are given in the `power_coefficient_polynomial` field, in increasing order, starting with the constant term. The absolute value of the spin ratio is used since the power does not depend on the direction of rotation. 

The default coefficients are taken from equation (9) in the article "[Design, operation and analysis of wind-assisted cargo ships](https://doi.org/10.1016/j.oceaneng.2020.107603)", by Tillig and Ringsberg (2020):

\\[
    C_P = 0.0234 \, SR - 0.0168 \, SR^2 + 0.0143 \, SR^3 - 0.0004 \, SR^4 + 0.0001 \, SR^5
\\]

The article states that the model is valid for rotors with an aspect ratio of 6 and an end disk diameter of two times the rotor diameter. In the article, the model is used for the whole rotor, together with the apparent wind velocity. In Stormbird, it is applied to each line element, with the local velocity, which is a strip theory assumption. Note also that the spin ratio is zero when the velocity is zero, so the model gives no power for a spinning rotor in still air. 

The power coefficient is used by the default [input power model](../line_model/input_power.md). Rotor sails can also use a power model that interpolates the power directly from the revolutions per second, which is independent of this coefficient.
