

# Corrections

The accuracy of an actuator line model is sensitive to both the force projection (shape and width), the mesh resolution in the CFD simulation, and the velocity sampling method. First, just to clarify, it is generally thought that wings will be simulated with a similar accuracy as a lifting line model IF the projection width is set to be sufficiently small, AND the mesh resolution is sufficiently high to allow for a small projection width. That is, an **actuator line model should converge towards a lifting line model** when the projection width is reduced.

However, at least when using isotropic force projections, it is often impractical to have a small enough force projection width to allow for this. This is especially the case as the whole point of using an actuator line model, rather than a direct CFD simulation, is to significantly reduce the number of cells used in a simulation. As such, it is fairly **common to combine an actuator line model with some form of correction** to the *raw circulation distribution*, to achieve better accuracy at coarse meshes. In particular, the main issue with an actuator line model is typically that the **tip circulation is predicted to be too large**. Or, in other words, the lift-induced velocities on the tip is too small.

As explored in the paper by [Churchfield et al., 2017](../literature/simulation_methods.md#an-advanced-actuator-line-method-for-wind-energy-applications-and-beyond-2017), this is an issue that might be also be solved by using force projection shapes that are more physically representative of a real wing, i.e. like with anisotropic kernels. However, other techniques are also often used, and the exact effect of the anisotropic force projection width deserves more investigation.

The Stormbird  library currently implements two different correction methods based on suggestions from the literature.

## Lifting line correction for the induced velocity

### Background
The first, and perhaps most promising, correction technique comes from the paper by [Dag and Sørensen, 2020](../literature/simulation_methods.md#a-new-tip-correction-for-actuator-line-computations-2020). The idea is based on an important observation: the lift-induced velocities from an actuator line model with a certain projection width, and an isotropic force projection kernel, seem to almost perfectly match the lift-induced velocity from a lifting line model with the same circulation distribution and a **viscous core length equal to the projection width**. That is, the vortices created by an actuator line model is like a smoothed, viscous, version of a pure potential theory vortex. The solution from this observation, suggested by Dag and Sørensen, is then to compute the error in the velocity field explicitly using a simple lifting line model, that directly quantify the effect of the viscous core.

For each time step in the simulation, the circulation strength from the previous time step is known. These strength values are also the values used when projecting the forces at the previous time step, and the strength that was active when the velocity from the CFD domain was sampled. A correction to the sampled velocity can then be computed based on the difference in the induced velocity from a lifting line model with and without a viscous core.

This is done by first computing the lift-induced velocities from a lifting line wake model with a viscous core equal to the projection width, and then doing the same calculation with very small core length. The difference in the induced velocity between the two wake models at each control point can then be used to correct the sampled velocity from the CFD grid.

### Wake shape
One challenge with such an approach is that the shape of the wake must be assumed somehow. This is a trivial task if there is only a single wing, but more difficult if there are multiple wings in close proximity OR wings standing on a ship superstructure. However, the point of the lifting line corrections is **only to compute the error due to a too large projection width**. The effect of a viscous core on the induced velocities are most significant from the wake close to the wing. The effect from the vortices far downstream should be small. The exact wake shape for the correction may, therefore, not be that important.

As a simple solution to this, the lifting line correction in Stormbird assumes a steady wake shape, with a direction equal to the average sampled velocity over each wing. The correction for each wing is also simulated independently. That is, **the correction should NOT affect interaction effects between wings in any way**, only errors due to a too large viscous core on the wings own wake. This is choice is made to keep the correction method local to a single wing. In other words, the lifting line correction only correct for errors in the *self-induced* velocities, and not interaction effects.

The actual code used to calculate the correction for a given time step is the same as the steady [wake models](../lifting_line/wake.md) used in the lifting line methods.

### Input structure
To activate the lifting line correction, set the a value for `lifting_line_correction` in the `ActuatorLineBuilder`. The available options are shown below:

```rust
pub struct LiftingLineCorrectionBuilder {
    pub wake_length_factor: f64,
    pub symmetry_condition: SymmetryCondition
}
```

The `wake_length_factor` is set to 100.0 by default, which means that the wake used for computing lift-induced velocities is assumed to extend 100 chord lengths downstream. The `symmetry_condition` is set to `NoSymmetry` by default. The symmetry condition should generally reflect the same symmetry condition as used in the CFD simulation. For instance, if the bottom of the domain is at the lowest z-coordinate, the symmetry condition should be set to `Z`.

Experience from this correction technique is that it works very well for actuator line simulations that only includes wings. Even with a **large isotropic projection width**, the result from an actuator line simulation will match the lifting line model almost perfectly. However, it can also create instabilities when combined with complex ship geometries. **The hope is to find a solution to this in the future**, as this type of correction is believed to be very general and promising for practical use cases.

## Empirical circulation correction

A more classical correction technique for actuator line simulations is to use an empirical correction factor to the calculated circulation on each wing. As mentioned, the *raw circulation at the tips* is often predicted to be too large. A simple solution is, then, to artificially reduce it based on a analytical correction. There are many small variations in how this is done. A recent discussion about the topic can be found in the paper by [Wimshurst and Willden, 2018](../literature/simulation_methods.md#spanwise-flow-corrections-for-tidal-turbines-2018). The correction in Stormbird follows the following principle:

The raw circulation distribution, \\( \Gamma_{raw} \\), estimated directly from the sampled velocity, is multiplied by a correction function, \\( f_{cor}(s) \\), that takes the non-dimensional span distance at each wing, \\( s \\) as input, as well as a shape parameter \\( \beta \\). The function has the following shape:

\\[
  f_{cor}(s) = \frac{2.0}{\pi}\cos^{-1}(e^{-\beta (0.5 - |s|)})
\\]

The plot below shows how the correction function looks for different values of \\( \beta \\). In general, any value of \\( \beta \\) will force the circulation towards zero at the tips. However, the larger the value, the steeper the reduction towards the tip, and the less effect it has on the inner part of the wing.

<script src="https://cdn.plot.ly/plotly-2.35.2.min.js"></script>
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

The `exp_factor` corresponds to the \\( \beta \\) variable in the equation above. It is set to 10.0 by default, but should ideally always be tuned for the specific case. The idea with this type of correction is to first tune the shape parameter based on simulation of a single wing, and then assume that it can remain constant also when simulating multiple wings or wings on a ship.

The `overall_correction` is a global correction factor applied to the entire circulation distribution, that can be used to either increase or decrease the overall circulation level. It is set to 1.0 by default.
