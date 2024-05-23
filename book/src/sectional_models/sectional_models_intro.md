# Section models

The primary purpose of the sectional models is to compute non-dimensional lift and drag as a function of the local flow velocity at each each line element. That is, they are two-dimensional models which is used together with the chord vector and local flow velocity to compute the total force on each line section. In both lifting line and actuator line simulations, three-dimensional effects are modeled by altering the effective velocity experienced by these two-dimensional models. 

Different sectional models are necessary for different sail types. This is handled by implementing the sectional model as an enum, with a definition as shown below:

```rust
pub enum SectionModel {
    Foil(Foil),
    VaryingFoil(VaryingFoil),
    RotatingCylinder(RotatingCylinder),
}
```

Each variant in this enum has its own sub-chapter for more details. A short overview is given below:

1) `Foil` represents a model for a single element foil profile, and is suitable for modelling single element wing sails
2) `VaryingFoil` is a model that extends the `Foil` model to allow the output to be dependent on some internal variable. The internal variable can for instance be a flap angle, for a two-element foil, a combination of multiple element configurations, for instance to model a three-element foil, or suction rate when modelling a suction sail.
3) `RotatingCylinder` represent a cylinder where the rotational speed can be varied to alter the force output. This is intended to be used to model rotor sails.

More sectional models can be added in the future. The only requirement is that each model must be able to compute the necessary force coefficients. However, the goal is to cover most use cases with these three core models. Since the models are handled through an enum, they may require different inputs in their own functions for calculating lift and drag. For instance, the `Foil` and `VaryingFoil` models require the angle of attack as input, while the `RotatingCylinder` takes the velocity magnitude and local chord length (or diameter) as input. The right input is managed by the line force model structure, and is not something the user needs to think about when running a simulation.

To get an overview of how to set up the different variants, see the sub-chapters for each model.