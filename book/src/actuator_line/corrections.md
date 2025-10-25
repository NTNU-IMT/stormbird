

# Corrections

The exact accuracy of an actuator line model is sensitive to both the force projection (shape and width), the mesh resolution in the CFD simulation, and the velocity sampling method. It is generally thought that a single wing will be represented in a similar way as with a lifting line model if the projection width is set to be sufficiently small, AND the mesh resolution is sufficiently high to allow for a small projection width. However, at least when using isotropic force projections, it is often impractical to have a small enough force projection width. As explored in the paper by [Churchfield et al., 2017](../literature/simulation_methods.md#an-advanced-actuator-line-method-for-wind-energy-applications-and-beyond-2017), this is an issue that might be solved by using force projection shapes that are more physically representative of a real wing, i.e. like with anisotropic kernels. However, other techniques are also often used. The Stormbird  library currently implements two different correction methods based on suggestions from the literature.

## Lifting line correction for the induced velocity

### Background
The first, and perhaps most promising, correction technique is based on the suggestion in the paper by [Dag and Sørensen, 2020](../literature/simulation_methods.md#a-new-tip-correction-for-actuator-line-computations-2020). The idea is based on an important observation: the lift-induced velocities from an actuator line model with a certain projection width, and isotropic force projection kernel, seem to almost perfectly match the lift-induced velocity from a lifting line model with the same circulation distribution and a viscous core length equal to the projection width. That is, the vortices created by an actuator line model is like a smoothed, viscous, version of a pure potential theory vortex.

Two solutions can be used with this observation in mind. The first one use a projection width that is sufficiently small that the viscous core of the projected vortices are negligible. However, this is generally not practical. The second, suggested by Dag and Sørensen, is to compute the error in the velocity field explicitly using a simple lifting line model. This is done by first running a simulation with a viscous core equal to the projection width, and then running a second simulation with a very small core length. The difference in velocity at each control point can then be used to correct the sampled velocity from the CFD grid.

One issue with such an approach is that the shape of the wake must be assumed somehow. This is a trivial task if there is only a single wing, but more difficult if there are multiple wings on close proximity OR wings standing on a ship superstructure. However, the point of the lifting line corrections is **only to compute the error due to a too large projection width**. The effect of a viscous core on the induced velocities are also most signficant close to the wing. The difference far downstream should be small. The exact wake shape for the correction may, therefore, not be that important.

The lifting line correction in Stormbird assumes a steady wake shape, with a direction equal to the average sampled velocity over each wing. The correction for each wing is also simulated independently. That is, **the correction should NOT affect interaction effects between wings in any way**, only errors due to a too large viscous core on the wings own wake.

The actual code used to calculate the correction for a given time step is the same as the steady [wake models](../lifting_line/wake.md) used in the lifting line methods.

### Input structure
To activate the lifting line correction, set the a value for 'lifting_line_correction' in the `ActuatorLineBuilder`. The available options are shown below:

```rust
pub struct LiftingLineCorrectionBuilder {
    pub wake_length_factor: Float,
    pub symmetry_condition: SymmetryCondition
}
```

The `wake_length_factor` is set to 100.0 by default. The `symmetry_condition` is set to `NoSymmetry` by default. The symmetry condition should generally reflect the same symmetry condition as used in the CFD simulation. For instance, if the bottom of the domain is at the lowest z-coordinate, the symmetry condition should be set to `Z`.

Experience from this correction technique is that it works very well for actuator line simulations that only includes wings. Even with a large projection width, the result from an actuator line simulation will match the lifting line model almost perfectly. However, it also seems to be somewhat unstable when combined with complex ship geometries. The hope is to find a solution to this in the future, as this type of correction is believed to be very general.

## Empirical circulation correction

A more classical way to correct for inaccuracies in an actuator line model is to apply empirical correction factors to the calculated circulation on each wing.

The *raw tip circulation* is often predicted to be too large, so a simple solution is to artificially reduce it based on a analytical correction shape. There are many small variations in how this is done. A recent discussion about the topic can be found in the paper by [Wimshurst and Willden, 2018](../literature/simulation_methods.md#spanwise-flow-corrections-for-tidal-turbines-2018).

The correction in Stormbird follows the following principle:

The raw circulation distribution, \\( \Gamma_{raw} \\), estimated directly from the sampled velocity, is multiplied by a correction function, \\( f_{cor}(s) \\), that takes the non-dimensional span distance at each wing, \\( s \\) as input. The function has the following shape:

\\[
  f_{cor}(s) = \frac{2.0}{\pi}\cos^{-1}(e^{-\beta (0.5 - |s|)})
\\]

<script src="https://cdn.plot.ly/plotly-latest.min.js"></script>
<div id="circulation-correction-plot"></div>
<script src="plot_scripts/circulation-correction-plot.js"></script>

### Input structure

To active this correction, the `EmpiricalCirculationCorrection` structure must be set in the `ActuatorLineBuilder`, as shown below:

```rust
pub struct EmpiricalCirculationCorrection {
    pub exp_factor: f64,
    pub overall_correction: f64,
}
```

The `exp_factor` corresponds to the \\( \beta \\) variable in the equation above. It is set to 21.0 by default. The `overall_correction` is a global correction factor applied to the entire circulation distribution, that can be used to either increase or decrease the overall circulation level. It is set to 1.0 by default.
