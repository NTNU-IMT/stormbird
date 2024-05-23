# Foil model

The single element foil model is a parametric model of a foil section. That is, it is defined using a limited set of parameters that are later used in a simple mathematical model to compute lift and drag for arbitrary angles of attack.

## Why a parametric model?
Other implementations of lifting line and actuator lines often use data based models for computing the lift and drag. That is, the user must supply data on how the lift and drag varies as a function of the angle attack, and then the solver can use this data together with interpolation or table look-up to compute force coefficients for arbitrary angles. 

A data based approach is often fine, and does have some benefits. There might be implementations of pure data based models in Stormbird in the future. However, the choice of using parametric model where based on two core reason. 

First, it becomes easier to use a parametric model as a building block for more complex foil models, where the behavior depends on some internal state, such as flap angle or suction rate, because the model parameters can be allowed to depend on the internal state through interpolation. See the [varying foil sub chapter](./varying_foil_model.md) for more on this.

Second, a parametric model ensures smoothness, which is beneficial when using the model together with gradient based optimization algorithms. For instance, such a method might be used to maximize thrust for a given wind direction. The smoothness is in particular practical when the expected optimal point is close to the stall angle. 

## Model overview
The model is divided in two core sub-models

1) For angles of attack below stall, it is assumed that both lift and drag can be represented accurately as simple polynomials. The lift is mostly linear, but can also have an optional high-order term where both the factor and power of the term is adjustable. The drag is assumed to be represented as a second order polynomial.
2) For angles of attack above stall, both the lift and drag are assumed to be harmonic functions which primarily is adjusted by setting the *max value* after stall. This is a rough model, which is assumed to be appropriate as the pre-stall behavior is usually more important for a wind power device.

The transit between the two models is done using a sigmoid function, where both the transition point and the width of the transition can be adjusted.

In addition, there factors in the model to account for added mass and lift due to the time derivative of the angle of attack. Both these effects are assumed to be linear for simplicity.

## Available fields

```rust

pub struct Foil {
    /// Lift coefficient at zero angle of attack. This is zero by default, but can be set to a 
    /// non-zero value to account for camber, flap angle or boundary layer suction/blowing.
    pub cl_zero_angle: f64,
    /// How fast the lift coefficient increases with angle of attack, when the angle of attack is
    /// small. The default value is 2 * pi, which is a typical value for a normal foil profile, 
    /// but it can also be set to different value for instance to account for boundary layer 
    /// suction/blowing.
    pub cl_initial_slope: f64,
    /// Optional proportionality factor for adding higher order terms to the lift. Is zero by 
    /// default, and therefore not used. Can be used to adjust the behavior of the lift curve close
    /// to stall.
    pub cl_high_order_factor: f64,
    /// Option power for adding higher order terms to the lift. Is zero by default, and therefore 
    /// not used. Can be used to adjust the behavior of the lift curve close to stall.
    pub cl_high_order_power: f64,
    /// The maximum lift coefficient after stall. 
    pub cl_max_after_stall: f64,
    /// Drag coefficient at zero angle of attack
    pub cd_zero_angle: f64,
    /// Factor to give the drag coefficient a second order term. This is zero by default.
    pub cd_second_order_factor: f64,
    /// The maximum drag coefficient after stall.
    pub cd_max_after_stall: f64,
    /// Power factor for the harmonic dependency of the drag coefficient after stall. Set to 1.6 by 
    /// default.
    pub cd_power_after_stall: f64,
    /// The mean stall angle, which is the mean angle where the model transitions from pre-stall to
    /// post-stall behavior. The default value is 20 degrees.
    pub mean_stall_angle: f64,
    /// The range of the stall transition. The default value is 6 degrees.
    pub stall_range: f64,
    /// Factor to model lift due to the time derivative of the angle of attack. This is zero by 
    /// default, and therefore not used.
    pub cl_changing_aoa_factor: f64,
    /// Factor to model added mass due to accelerating flow around the foil. Set to zero by default.
    pub added_mass_factor: f64,
}
```
