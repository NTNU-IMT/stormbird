# Circulation strength

Wings generate forces in several ways, as further explained in the [force calculation chapter](./force_calculations.md). However, the main force is the lift which depends on the circulation strength. This chapter specifies how the circulation strength is estimated from the local velocity on each line section, and how it is possible to modify the estimation. The procedure is the same for all simulation methods in Stormbird. That is, there are no differences in this regard between the static lifting line, dynamic lifting line or the actuator line. The responsibility for calculating the values are therefore given to the line force model. The circulation distribution will also affect the velocity in the simulation domain, either through induced velocities from potential theory wake models in the case of lifting line simulations, or through body forces projected into a CFD domain in the case of actuator line simulations. 

## Raw lifting line theory estimation
Equations and explanations to come!

View in the to source code below:

```rust
pub fn circulation_strength_raw(&self, velocity: &[Vec3]) -> Vec<f64> {
    let cl = self.lift_coefficients(&velocity);

    (0..velocity.len()).map(|index| {
        -0.5 * self.chord_vectors_local[index].length() * velocity[index].length() * cl[index] * self.density
    }).collect()
}
```

## Optional smoothing for difficult cases

Noise in the estimated circulation strength can sometimes happen, and this might cause instabilities and error in the estimated forces. Typical examples are lifting line simulations of stalled wings - at least when the wake is quasi-static - and actuator line simulations with very large lift-coefficients - such as for rotor sails.

To handle such cases in a practical manner, there are optional *smoothing* methods that can be used when estimating the circulation strength. These methods are controlled through a `SmoothingSettings` structure that is specified for the line force model. The fields in the complete structure is given below:

```rust
pub struct SmoothingSettings {
    pub gaussian: Option<GaussianSmoothingSettings>
    pub artificial_viscosity: Option<ArtificialViscositySettings>,
}
```

These fields control two different smoothing methods, which are further explained below. All parameters are optional and set to `None` by default, meaning that none of the methods are used unless they are activated by the user. 

### Gaussian smoothing
The `GaussianSmoothingSettings` contains parameters used to apply a [Gaussian smoothing filter](https://en.wikipedia.org/wiki/Kernel_smoother) to the estimated circulation distribution. The available fields are shown below:

```rust
pub struct GaussianSmoothingSettings {
    pub length_factor: f64,
    pub end_corrections: Vec<(bool, bool)>,
}
```

The `length_factor` gives a factor used to calculated The smoothing length from the wing span of each wing in the line force model. That is, if the value is set to 0.01, the smoothing length will be 1% of the span of each wing, independent of the value of the wing span or the number of sections.

The `end_corrections` contains a vector - that should be equal in length to the number of wings - with a tuple of booleans that specifies whether or not the circulation distribution on the ends of the wing should be corrected for the fact that the ends typically have zero circulations.

An example of how this smoothing method affects the circulation distribution is illustrated in the figure below. **Note**: the example is with an excessive amount of noise, and is not representative of actual numerical noise from a lifting line simulation. Rather, it shows an example where an artificial elliptic circulation distribution was first generated, and then modified by adding random numerical noise. The plot then shows how the noise is reduced when the noisy circulation distribution is corrected using the Gaussian smoothing filer, with different values for the `gaussian_length_factor`. The end corrections are set to `true` in this case, as the circulation distribution is supposed to be zero at the ends. 

![Gaussian smoothing example](figures/gaussian_smoothing_example.png)

As can bee seen, simple Gaussian smoothing introduces some errors towards the end of the wings if the smoothing length is too large, although the random noise is effectively reduced. It is therefore generally recommended to only apply as little smoothing as necessary to stabilize a solution. 

### Artificial viscosity
This is currently an experimental feature and should be **used with care**. It adds a viscosity term to the estimated circulation distribution, based on the second derivative of the circulation as a function of span location multiplied with this parameter. The idea is taken from this [pre-print](https://www.researchgate.net/publication/378262301_An_Efficient_3D_Non-Linear_Lifting-Line_Method_with_Correction_for_Post-Stall_Regime). The paper suggest that an artificial viscosity term can in some cases stabilize the results in challenging conditions for the solver. Similar results are found when using the same approach in Stormbird. However, the parameter requires careful tuning to work properly, and can quickly also lead to instabilities. A particular problem is that both too low and too high values may cause instabilities. At the moment, Gaussian smoothing is recommended for cases with unstable results.

Available fields:

```rust
pub struct ArtificialViscositySettings {
    pub viscosity: f64,
    pub iterations: usize,
}
```

More will come if we find good recommendations for the settings. If not, the option might be removed... 

## Predetermined distributions

To come!