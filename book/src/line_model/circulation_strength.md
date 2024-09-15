# Circulation strength

Wings generate forces in several ways, as further explained in the [force calculation chapter](./force_calculations.md). However, the main force is the lift which is calculated from the circulation strength. This chapter specifies how the circulation strength is estimated from the local velocity on each line section, and how it is possible to modify the estimation for stability purposes. The procedure is the same for all simulation methods in Stormbird. That is, there are no differences in this regard between the static lifting line, dynamic lifting line or the actuator line. The responsibility for calculating the values are therefore given to the line force model.

The circulation distribution will also affect the velocity in the simulation domain, either through induced velocities from potential theory wake models, in the case of lifting line simulations, or through body forces projected into a CFD domain, in the case of actuator line simulations. The circulation strength for each line element is therefore a core variable in any simulation using line force models.

## Raw lifting line theory estimation
The calculation of the circulation strength on each line element follows the [Kutta–Joukowski theorem](https://en.wikipedia.org/wiki/Kutta%E2%80%93Joukowski_theorem). The mathematical definition of the circulation value, \\( \Gamma \\), on a line element is as follows, where \\(U \\) is the velocity, and \\( L \\) is the lift per unit span [^gamma_definition_note]

\\[
    \Gamma = L / (U \rho)
\\]

The lift per unit span is further computed from the sectional lift coefficient, \\(C_L\\) as follows, where \\(\rho\\) is the density and \\(c \\) the chord length:

\\[
    L = 0.5 \cdot \rho \cdot c \cdot C_L \cdot U^2 
\\]

When these equations are combined, we get the following equation for the circulation strength:

\\[
    \Gamma = 0.5 \cdot c \cdot C_L \cdot U 
\\]

The actual source code looks like the following (the negative values are to account for directional definitions)

```rust
pub fn circulation_strength_raw(&self, velocity: &[Vec3]) -> Vec<f64> {
    let cl = self.lift_coefficients(&velocity);

    (0..velocity.len()).map(|index| {
        -0.5 * self.chord_vectors_local[index].length() * velocity[index].length() * cl[index]
    }).collect()
}
```

## Optional corrections

Sometimes, there might be noise in the estimated circulation strength, which might cause instabilities and errors in the estimated forces. A typical examples is lifting line simulations of stalled wings - especially when the lift coefficient is very large.

To handle such cases in a practical manner, there are optional *corrections methods* that can be used when estimating the circulation strength. These methods are controlled through a `CirculationCorrection` enum that is specified for the line force model. The variants in the enum is given below:

```rust
pub enum CirculationCorrection {
    None, // The default
    PrescribedCirculation(PrescribedCirculationShape),
    GaussianSmoothing(GaussianSmoothing),
}
```

The default variant is `None`, which means that no corrections are applied. The effect of the other variants are explained below. 

### Gaussian smoothing

The `GaussianSmoothingSettings` contains parameters used to apply a [Gaussian smoothing filter](https://en.wikipedia.org/wiki/Kernel_smoother) to the estimated circulation distribution. The available fields are shown below:

```rust
pub struct GaussianSmoothingSettings {
    pub length_factor: f64,
}
```

The `length_factor` gives a factor used to calculated the smoothing length from the span of each wing in the line force model. That is, if the value is set to 0.01, the smoothing length will be 1% of the total span of each wing, independent of the value of the wing span or the number of sections.


An example of how this smoothing method affects the circulation distribution is illustrated in the figure below. **Note**: the example is with an excessive amount of noise, and is not representative of actual numerical noise from a lifting line simulation. Rather, it shows an example where an artificial elliptic circulation distribution was first generated, and then modified by adding random numerical noise. The plot then shows how the noise is reduced when the noisy circulation distribution is corrected using the Gaussian smoothing filer, with different values for the `gaussian_length_factor`.

![Gaussian smoothing example](figures/gaussian_smoothing_example.png)

As can bee seen, simple Gaussian smoothing introduces some errors towards the end of the wings if the smoothing length is too large, although the random noise is effectively reduced. It is therefore generally recommended to only apply as little smoothing as necessary to stabilize a solution. 

## Prescribed distribution

Predetermined circulation distributions are a special mode where the circulation is forces to always follow a simple mathematical shape. For instance, it is possible to force the distribution to always be elliptical. This gives very stable simulations, and sometimes results that are very close a *full simulation*. It is particular useful if the goal is only to estimate interaction effects between wings, but where the lift and drag for a single wing is already known from, for instance, experimental or CFD results. **more on this to come**.

A view of the `PrescribedCirculationShape` structure: 

```rust
pub struct PrescribedCirculationShape {
    pub inner_power: f64,
    pub outer_power: f64,
}
```