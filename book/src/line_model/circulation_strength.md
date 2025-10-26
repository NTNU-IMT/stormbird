# Circulation strength

The circulation strength along a wing determines both the magnitude of the lift-forces and the induced velocities from the wing. The velocity is affected either through induced velocities from a wake model based on potential theory, in the case of lifting line simulations, or through body forces projected into a CFD domain, in the case of actuator line simulations.

This chapter specifies how the circulation strength is estimated from the local velocity on each line section, and how it is possible to modify the estimation for stability purposes. The procedure is the same for all simulation methods in Stormbird. That is, there are no differences in this regard between the static lifting line, dynamic lifting line or the actuator line. The responsibility for calculating the values are therefore given to the line force model.

## Raw lifting line theory estimation
The calculation of the circulation strength on each line element follows the [Kutta–Joukowski theorem](https://en.wikipedia.org/wiki/Kutta%E2%80%93Joukowski_theorem). The mathematical definition of the circulation value, \\( \Gamma \\), on a line element is as follows, where \\(U \\) is the velocity, and \\( L \\) is the lift per unit span.

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

To handle such cases in a practical manner, there are optional *corrections methods* that can be used when estimating the circulation strength. These methods are controlled through a `CirculationCorrection` enum that is specified for the line force model. To set up different types of corrections, a `CirculationCorrectionBuilder` is used, which has the following structure:

```rust
pub enum CirculationCorrectionBuilder {
    None,
    Prescribed(PrescribedCirculation),
    Smoothing(CirculationSmoothingBuilder),
}
```

The default variant is `None`, which means that no corrections are applied. The effect of the other variants are explained below.

### Smoothing

The smoothing correction applies different types of smoothing filters to the estimated circulation, controlled through the following structure:

```rust
pub struct CirculationSmoothingBuilder {
    pub smoothing_type: SmoothingTypeBuilder,
    pub prescribed_to_subtract_before_smoothing: Option<PrescribedCirculation>,
}
```

The option to add a `prescribed_to_subtract_before_smoothing` is currently. It subtracts a prescribed circulation distribution (see more about this below) from the estimated circulation before applying the smoothing filter. After the smoothing is applied, the prescribed distribution is added back to the smoothed result. The idea behind this is that it can avoid issues with smoothing the rapidly changing circulation at the wing tips. By subtracting a known distribution first, the smoothing filter only needs to handle the more noisy part of the circulation distribution. However, the this feature is still experimental, and should be used with care.

The `smoothing_type` field specifies which type of smoothing filter should be applied. The available options are shown below:

```rust
pub enum SmoothingTypeBuilder {
    Gaussian(GaussianSmoothingBuilder),
    CubicPolynomial(CubicPolynomialSmoothingBuilder),
}
```

The first smoothing type is [Gaussian smoothing filter](https://en.wikipedia.org/wiki/Kernel_smoother). The available fields are shown below:

```rust
pub struct GaussianSmoothingBuilder {
    pub smoothing_length_factor: f64,
    pub number_of_end_points_to_interpolate: usize
}
```

The `smoothing_length_factor` gives a factor used to calculated the smoothing length from the span of each wing in the line force model. That is, if the value is set to 0.01, the smoothing length will be 1% of the total span of each wing, independent of the value of the wing span or the number of sections.

The `number_of_end_points_to_interpolate` field specifies how many points at each end of the wing that should be inserted beyond the tips of the wings before applying the smoothing. Which values that are inserted beyond the tips are dependent on wether the circulation is expected to be zero or not, controller by the `non_zero_circulation_at_ends` field in the line force model. If the circulation is expected to have zero circulation, the additional points are set to zero. If not, the values are linearly extrapolated from the inner points. If this value is not set, the default will be to calculate the number of insertion points based on the smoothing length and the number of sections. This should be sufficient in most cases.


An example of how this smoothing method affects the circulation distribution is illustrated in the figure below. **Note**: the example is with an excessive amount of noise, and is not representative of actual numerical noise from a lifting line simulation. Rather, it shows an example where an artificial elliptic circulation distribution was first generated, and then modified by adding random numerical noise. The plot then shows how the noise is reduced when the noisy circulation distribution is corrected using the Gaussian smoothing filer, with different values for the `gaussian_length_factor`.

![Gaussian smoothing example](figures/gaussian_smoothing_example.png)

As can bee seen, simple Gaussian smoothing introduces some errors towards the end of the wings if the smoothing length is too large, although the random noise is effectively reduced. It is therefore generally recommended to only apply as little smoothing as necessary to stabilize a solution.

The second smoothing type is a cubic polynomial smoothing filter, which assumes that a cubic polynomial can be fitted to the points with a certain `window_size`. The larger the window size, the more smoothing is applied. The available fields are shown below:

```rust
pub enum WindowSize {
    Five,
    Seven,
    Nine
}

pub struct CubicPolynomialSmoothingBuilder {
    pub window_size: WindowSize,
}
```

## Prescribed distribution

Predetermined circulation distributions are a special mode where the circulation is forces to always follow a simple mathematical shape. For instance, it is possible to force the distribution to always be elliptical. This gives very stable simulations, and sometimes results that are very close a *full simulation*. It is particular useful if the goal is only to estimate interaction effects between wings, but where the lift and drag for a single wing is already known from, for instance, experimental or CFD results.

A view of the `PrescribedCirculation` structure:

```rust
pub struct PrescribedCirculationShape {
    pub inner_power: Float,
    pub outer_power: Float,
}

pub struct PrescribedCirculation {
    pub shape: PrescribedCirculationShape,
    pub curve_fit_shape_parameters: bool,
}
```

The option to `curve_fit_shape_parameters` is currently experimental. It will curve fit the shape parameters to the raw circulation distribution before applying the prescribed shape. However, it currently ends up with a relatively slow simulation, and the final results are not always better than just using default values for the shape parameters. It is set to false by default, and tuning it on should be used with care and testing.

The `PrescribedCirculationShape` parameters in the structure is used to force the circulation to always follow a mathematical shape that looks like the equation below, where \\(s\\) is the local non-dimensional span distance, varying from -0.5 to 0.5 along each wing:

\\[
    \Gamma(s) = \Gamma_0 (1.0 - (2 s)^{\text{inner_power}})^{\text{outer_power}}
\\]

The default values are to set `inner_power` to 2.0 and `outer_power` to 0.5. This corresponds to an elliptic distributions.
