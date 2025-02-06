# Rotating cylinder
Model representing a rotating cylinder, where the main intended use case is to model rotor sails. The purpose is to calculate lift, drag on a two-dimensional cylinder as a function of how fast the cylinder is spinning.

The primary input variable to the model is the *spin ratio*, which is defined as the ratio of the velocity of the rotor to the wind velocity. The velocity of the rotor is defined as the circumference times the rotations per seconds. 

Unlike the [foil model](foil_model.md), the rotating cylinder model is entirely data based and not parametric. There are primary two reasons for this: 

1) The behavior of lift and drag on a spinning cylinder is, in some ways, simpler than a foil section. The lift will generally increase with increasing spin-ratios, and not experience stall in the same way as a foil. In addition, the two-dimensional drag is generally going from a high value at zero spin-ratio to a very low value at normal operating spin-ratios. A few data points are therefore often enough to capture the behavior of a rotating cylinder.
2) It is not common, at least not yet, to combine a rotating cylinder with other control mechanism. For instance, for wing sails and suction sails it is common to control the sails with both a flap angle or suction rate, and the angle of attack. A rotor sail is only controlled through its rotational speed. A simple one dimensional data model is therefore assumed to be sufficient for rotor sails (lift and drag as function of spin ratio only)

## Available parameters

The parameters in the rotating cylinder are listed below.

The `cl_data`, `cd_data` and `spin_ratio_data` is the most important parts of the model. They specify how lift and drag is dependent on the spin ratio. All variables have default variables based on the results in the article "[Calculation of Flettner rotor forces using lifting line and CFD methods](https://blueoasis.pt/wp-content/uploads/2023/10/Nutts2023_proceedings_v4.pdf)". See also the [validation data section](../literature/validation_data.md) for more on this.

The `added_mass_factor` and `moment_of_inertia` parameters can be used to estimate added mass forces and gyroscopic forces on the rotor. **Note**: more to come on these parameters later. They need further validation, and are therefore set to zero by default.

The spin ratio for each section is calculated based on the `revolutions_per_second` value and the local chord length and velocity. Then, the lift and drag in 2D is interpolated from the input data values. 

```rust
pub struct RotatingCylinder {
    pub revolutions_per_second: f64,
    pub spin_ratio_data: Vec<f64>,
    pub cl_data: Vec<f64>,
    pub cd_data: Vec<f64>,
    pub added_mass_factor: f64,
    pub moment_of_inertia_2d: f64,
}
```