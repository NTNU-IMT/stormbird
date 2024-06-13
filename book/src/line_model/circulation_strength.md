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

## Optional smoothing for difficult cases

Sometimes, there might be noise in the estimated circulation strength, which might cause instabilities and errors in the estimated forces. Typical examples are lifting line simulations of stalled wings - at least when the wake is quasi-static - and actuator line simulations with very large lift-coefficients - such as for rotor sails.

To handle such cases in a practical manner, there are optional *smoothing methods* that can be used when estimating the circulation strength. These methods are controlled through a `SmoothingSettings` structure that is specified for the line force model. The fields in the complete structure is given below:

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

The `length_factor` gives a factor used to calculated the smoothing length from the span of each wing in the line force model. That is, if the value is set to 0.01, the smoothing length will be 1% of the totla span of each wing, independent of the value of the wing span or the number of sections.

The `end_corrections` contains a vector - that must be equal in length to the number of wings - with a tuple of booleans that specifies whether or not the circulation distribution on the ends of the wing should be corrected for the fact that the ends typically have zero circulations. The correction happens by artificially inserting zero values at both ends of the input vector to the Gaussian smoothing method.

An example of how this smoothing method affects the circulation distribution is illustrated in the figure below. **Note**: the example is with an excessive amount of noise, and is not representative of actual numerical noise from a lifting line simulation. Rather, it shows an example where an artificial elliptic circulation distribution was first generated, and then modified by adding random numerical noise. The plot then shows how the noise is reduced when the noisy circulation distribution is corrected using the Gaussian smoothing filer, with different values for the `gaussian_length_factor`. The end corrections are set to `true` in this case, as the circulation distribution is supposed to be zero at the ends. 

![Gaussian smoothing example](figures/gaussian_smoothing_example.png)

As can bee seen, simple Gaussian smoothing introduces some errors towards the end of the wings if the smoothing length is too large, although the random noise is effectively reduced. It is therefore generally recommended to only apply as little smoothing as necessary to stabilize a solution. 

### Artificial viscosity
This concept is Based on the work first presented in Chattot (2004), but also used in other papers such as in Gallay et al. (2015) and Simonet et al. (2024). 

It is found that it is possible to stabilize the solution of the circulation distribution in non-linear cases by adding an *artificial viscosity term* that is dependent on the second derivative of the distribution. 

That is, if the raw circulation strength from the original lifting line theory at line element i is called \\(\Gamma_{i, 0} \\) and the span distance of element i is labeled \\(s\\), then we can calculate a corrected circulation strength, \\( \Gamma_i \\), using an artificial viscosity parameter \\(\mu\\), as follows:

\\[
    \Gamma_i = \Gamma_{i,0} + \mu \frac{\partial^2\Gamma_i}{\partial s^2}
\\]

The double derivative of the circulation distribution can be calculated using finite difference. 

However, a challenge is that the solution of \\(\Gamma_i \\) also depends on the second derivative of itself. A solver is therefore necessary to find the correction term. 

At the moment, this is handled using the same type on dampened iterative solver as for the raw circulation strength[^solver_note]. 

The available fields for this solver is given below:

```rust
pub struct ArtificialViscositySettings {
    pub viscosity: f64,
    pub solver_iterations: usize,
    pub solver_damping: f64
}
```

[^solver_note]: It seems like there should be a more elegant solution to this solver. However, this has not been prioritized yet.

## Predetermined distributions

Predetermined circulation distributions are a solution that allow the simulation to directly use lift and drag data from some external source for a single wing - e.g., CFD or experimental data - but still be able to model interaction effects between several wings in a simplified manner. This solution is particularly useful for rotor sails (see the rotor sail tutorial for more), or for any other cases with complex flow around the wings that are not modelled accurately directly by the lifting line method.

More to come!

## References
- Chattot, J., 2004. Analysis and design of wings and wing/winglet combinations at low speeds. Computational Fluid Dynamics. Available [here](https://arc.aiaa.org/doi/10.2514/6.2004-220)
- Gallay, S., Laurendeau, E., 2015. Nonlinear Generalized Lifting-Line Coupling Algorithms for Pre/Poststall Flows. AiAA Journal. Available [here](https://www.researchgate.net/publication/276123891_Nonlinear_Generalized_Lifting-Line_Coupling_Algorithms_for_PrePoststall_Flows)
- Simonet, T., Roncin, K., Faure, T. M., Daridon, L., 2024. An efficient 3D non-linear lifting-line method with correction for post-stall regime. Preprint. Available [here](https://www.researchsquare.com/article/rs-3955527/v1)