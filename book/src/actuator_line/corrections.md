

# Corrections

The accuracy of an actuator line model is sensitive to both the force projection (shape and width), the mesh resolution in the CFD simulation, and the velocity sampling method. First, just to clarify, it is generally thought that wings will be simulated with a similar accuracy as a lifting line model IF the projection width is set to be sufficiently small, AND the mesh resolution is sufficiently high to allow for a small projection width. That is, an **actuator line model should converge towards a lifting line model** when the projection width is reduced.

However, at least when using isotropic force projections, it is often impractical to have a small enough force projection width to allow for this. This is especially the case as the whole point of using an actuator line model, rather than a direct CFD simulation, is to significantly reduce the number of cells used in a simulation. As such, it is fairly **common to combine an actuator line model with some form of correction** to the *raw circulation distribution*, to achieve better accuracy at coarse meshes. In particular, the main issue with an actuator line model is typically that the **tip circulation is predicted to be too large**. Or, in other words, the lift-induced velocities on the tip is too small.

As explored in the paper by [Churchfield et al., 2017](../literature/simulation_methods.md#an-advanced-actuator-line-method-for-wind-energy-applications-and-beyond-2017), this is an issue that might also be solved by using force projection shapes that are more physically representative of a real wing, i.e. like with anisotropic kernels. However, other techniques are also often used, and the exact effect of the anisotropic force projection width deserves more investigation.

The Stormbird library currently implements two different correction methods based on suggestions from the literature: a lifting line correction of the sampled velocity, and an empirical tip loss correction of the circulation.

## Lifting line correction for the induced velocity

### Background
The first, and perhaps most promising, correction technique comes from the paper by [Dag and Sørensen, 2020](../literature/simulation_methods.md#a-new-tip-correction-for-actuator-line-computations-2020). The idea is based on an important observation: the lift-induced velocities from an actuator line model with a certain projection width, and an isotropic force projection kernel, seem to almost perfectly match the lift-induced velocity from a lifting line model with the same circulation distribution and a **viscous core length equal to the projection width**. That is, the vortices created by an actuator line model are like a smoothed, viscous, version of a pure potential theory vortex. The solution from this observation, suggested by Dag and Sørensen, is then to compute the error in the velocity field explicitly using a simple lifting line model, that directly quantifies the effect of the viscous core.

For each time step in the simulation, the circulation strength from the previous time step is known. These strength values are also the values used when projecting the forces at the previous time step, and the strength that was active when the velocity from the CFD domain was sampled. A correction to the sampled velocity can then be computed based on the difference in the induced velocity from a lifting line model with and without a viscous core.

This is done by first computing the lift-induced velocities from a lifting line wake model with a viscous core equal to the projection width, and then doing the same calculation with very small core length. The difference in the induced velocity between the two wake models at each control point can then be used to correct the sampled velocity from the CFD grid.

Normally, the same circulation distribution is used in both wake models. The exception is when the [tip loss correction](#tip-loss-correction) is only applied to the projected forces. The circulation used for the projected forces then differs from the circulation used for the forces on the wings. The wake model with the viscous core represents what is induced in the CFD domain, and therefore uses the projected circulation, while the wake model with the small core uses the circulation from the forces on the wings. See the [section on the application of the tip loss correction](#application-to-the-projected-forces-only) for more.

### Wake shape
One challenge with such an approach is that the shape of the wake must be assumed somehow. This is a trivial task if there is only a single wing, but more difficult if there are multiple wings in close proximity OR wings standing on a ship superstructure. However, the point of the lifting line corrections is **only to compute the error due to a too large projection width**. The effect of a viscous core on the induced velocities is most significant from the wake close to the wing. The effect from the vortices far downstream should be small. The exact wake shape for the correction may, therefore, not be that important.

As a simple solution to this, the lifting line correction in Stormbird assumes a steady wake shape, with a direction equal to the sampled velocity along each wing.

### Isolated wings or wing interactions
By default, the correction for each wing is simulated independently. That is, **the correction does NOT affect interaction effects between wings in any way**, only errors due to a too large viscous core on the wing's own wake. This choice is made to keep the correction method local to a single wing, under the assumption that interaction effects are better predicted directly by the actuator line model itself. In other words, the default lifting line correction only corrects for errors in the *self-induced* velocities.

However, a too large viscous core may also cause errors in the wing-to-wing interactions, for instance when wings are placed close to each other. To account for this, the correction can optionally include all wings, by setting `include_wing_interactions` to `true` in the input structure (see below). The wakes of all wings are then included when computing the induced velocities in both wake models, so that the difference between them also contains the error in the velocities induced by the other wings. For a single wing, the two options give identical results.

The actual code used to calculate the correction for a given time step is the same as the steady [wake models](../lifting_line/wake.md) used in the lifting line methods.

### Input structure
To activate the lifting line correction, set a value for `lifting_line_correction` in the `ActuatorLineBuilder`. The available options are shown below:

```rust
pub struct LiftingLineCorrectionBuilder {
    pub wake_length_factor: f64,
    pub symmetry_condition: SymmetryCondition,
    pub initialization_time: Option<f64>,
    pub reference_core_length_factor: f64,
    pub include_wing_interactions: bool,
    pub max_correction_ratio: Option<f64>,
    pub virtual_extension_factors: [f64; 2],
}
```

The `wake_length_factor` is set to 100.0 by default, which means that the wake used for computing lift-induced velocities is assumed to extend 100 chord lengths downstream. The `symmetry_condition` is set to `NoSymmetry` by default. The symmetry condition should generally reflect the same symmetry condition as used in the CFD simulation. For instance, if the bottom of the domain is at the lowest z-coordinate, the symmetry condition should be set to `Z`.

The viscous core length used to represent the actuator line is calculated from the force projection settings, as the average of the `chord_factor` and `thickness_factor` in the [Gaussian projection kernel](force_projection.md) multiplied with the average chord length. The *very small* core length used in the second wake model is then calculated by dividing this value by the `reference_core_length_factor`, which is set to 100.0 by default.

The `initialization_time` is optional, and not used by default. If it is set, the correction is gradually ramped up from zero to its full value, using a smooth cosine transition, over the specified time from the start of the simulation. This can be used to avoid large corrections in the start-up phase of a CFD simulation.

The `include_wing_interactions` is set to `false` by default, meaning that the correction is computed for each wing in isolation. If set to `true`, the correction includes the induced velocities from all wings, as described in the [section above](#isolated-wings-or-wing-interactions).

Two additional settings are available, which are still experimental:

- `max_correction_ratio`: optional limit on the magnitude of the velocity correction at each control point, as a ratio of the magnitude of the sampled velocity. The direction of the correction is kept. Not used by default. This can be needed for wings with a large circulation at the ends, such as rotor sails, where the correction can otherwise become larger than the freestream velocity at the ends of the wing.
- `virtual_extension_factors`: length of a virtual extension of each wing at the first and the last end, relative to the chord length at that end. The extensions are only included in the reference model with the small viscous core, and carry the circulation of the outermost line element, which moves the tip vortices away from the ends of the wing. This can be used to represent end plates. Zero by default, which means no extensions.

Experience from this correction technique is that it works very well for actuator line simulations that only include wings. Even with a **large isotropic projection width**, the result from an actuator line simulation will match the lifting line model almost perfectly. However, it can also create instabilities when combined with complex ship geometries. **The hope is to find a solution to this in the future**, as this type of correction is believed to be very general and promising for practical use cases.

## Tip loss correction

A more classical correction technique for actuator line simulations is to use an empirical correction factor to the calculated circulation on each wing, often referred to as a tip loss correction. As mentioned, the *raw circulation at the tips* is often predicted to be too large. A simple solution is, then, to artificially reduce it based on an analytical correction. There are many small variations in how this is done. A recent discussion about the topic can be found in the paper by [Wimshurst and Willden, 2018](../literature/simulation_methods.md#spanwise-flow-corrections-for-tidal-turbines-2018). The correction in Stormbird follows the following principle:

The raw circulation distribution, \\( \Gamma\_{raw} \\), estimated directly from the sampled velocity, is multiplied by a correction function, \\( f\_{cor}(s) \\), that takes the non-dimensional span distance at each wing, \\( s \\) as input, as well as a shape parameter \\( \beta \\). The function has the following shape:

\\[
  f\_{cor}(s) = \frac{2.0}{\pi}\cos^{-1}(e^{-\beta (0.5 - |s|)})
\\]

The plot below shows how the correction function looks for different values of \\( \beta \\), which can be adjusted with the slider, or by typing in a value, below the plot. In general, any value of \\( \beta \\) will force the circulation towards zero at the tips. However, the larger the value, the steeper the reduction towards the tip, and the less effect it has on the inner part of the wing.

The plot also shows the corrected circulation, relative to the raw circulation, for a wing with a constant raw circulation along the span when the mean value is maintained, as explained in the [input structure section](#input-structure) below. The constant raw circulation is shown as a dashed line.

<link rel="stylesheet" href="../sectional_models/interactive/model_explorer.css">

<div data-tip-loss-correction-explorer>
    <noscript>The interactive plot requires JavaScript.</noscript>
</div>

<script src="../sectional_models/interactive/model_explorer.js"></script>
<script src="interactive/tip_loss_correction_explorer.js"></script>

### Input structure

To activate this correction, the `TipLossCorrection` structure must be set as the `tip_loss_correction` in the `ActuatorLineBuilder`. The available options are shown below:

```rust
pub struct TipLossCorrection {
    pub exp_factor: f64,
    pub maintain_mean_value: bool,
    pub application: TipLossApplication,
}

pub enum TipLossApplication {
    Circulation,
    ProjectedForces,
}
```

The `exp_factor` corresponds to the \\( \beta \\) variable in the equation above. It is set to 10.0 by default, but should ideally always be tuned for the specific case. The idea with this type of correction is to first tune the shape parameter based on simulation of a single wing, and then assume that it can remain constant also when simulating multiple wings or wings on a ship.

The `maintain_mean_value` flag is set to true by default. The correction function above is always less than one, so on its own, it reduces the mean circulation on each wing, and therefore also the total lift. When `maintain_mean_value` is true, the corrected circulation on each wing is instead scaled with a constant factor, so that the mean value over the span, weighted with the length of each line segment, is the same as before the correction:

\\[
  \Gamma\_{cor} = k \, f\_{cor} \, \Gamma\_{raw}, \quad k = \frac{\sum \Gamma\_{raw} \, l}{\sum f\_{cor} \, \Gamma\_{raw} \, l}
\\]

where \\( l \\) is the length of each line segment, and the sums are taken over all the line segments on the wing. The correction then only changes the shape of the circulation distribution, by moving circulation from the ends of the wing towards the middle. The idea is that the actuator line model is assumed to predict the overall level of the circulation well, while the circulation at the ends is too large. If the sum in the denominator is close to zero compared to the sum of the absolute values of the raw circulation, for instance when the lift on the wing is close to zero, the scaling is skipped to avoid dividing by a very small number. Setting `maintain_mean_value` to false gives the correction without any scaling.

The `application` field determines where the correction is applied. The default, `Circulation`, applies the correction to the estimated circulation, as described above. Both the forces on the wings and the forces projected to the CFD grid are then computed from the corrected circulation. The other option, `ProjectedForces`, is explained in the next section.

### Application to the projected forces only

> **Experimental:** this option is new, and has so far only been tested for rotor sails. The recommended settings may change.

The tip loss correction can also be used for a different purpose than correcting a too large circulation at the tips: to **stabilize** simulations of wings with a large circulation at the ends, such as rotor sails. The circulation on a rotor sail is almost constant along the span, which gives very strong tip vortices. When combined with the [lifting line correction](#lifting-line-correction-for-the-induced-velocity), this has been found to give unstable simulations. Reducing the circulation towards the ends smears the tip vortex over a larger part of the span, which stabilizes the simulation. However, if the correction is applied to the circulation itself, it also changes the forces on the wing. In particular, it reduces the lift-induced drag, as the circulation distribution is no longer as close to a constant distribution, which then must be corrected for in some other way.

Setting `application` to `ProjectedForces` separates the two effects. The circulation estimated from the sampled velocity, \\( \Gamma\_{raw} \\), is used for all the forces on the wings, and is the circulation that is written in the results. The tip loss correction is only applied to a copy of the circulation, \\( \Gamma\_{proj} \\), that is used for the forces projected to the CFD grid. As such, only the tip vortices in the CFD domain are smeared out. The correction is applied after the damping of the circulation in the [solver](simulation_overview.md#running-a-simulation), so that the damping only acts on the uncorrected circulation.

This option is primarily intended to be combined with the lifting line correction. The two wake models in the lifting line correction then use different circulation distributions:

\\[
  \mathbf{u}\_{cor} = \mathbf{u}\_{CFD} + \mathbf{u}\_{ref}(\Gamma\_{raw}) - \mathbf{u}\_{visc}(\Gamma\_{proj})
\\]

where \\( \mathbf{u}\_{ref} \\) is the induced velocity from the wake model with the small viscous core, and \\( \mathbf{u}\_{visc} \\) is the induced velocity from the wake model with the viscous core equal to the projection width. The idea is that the sampled velocity, \\( \mathbf{u}\_{CFD} \\), contains the induced velocities from the smeared, projected, circulation, which is removed by the last term. The corrected velocity is then approximately the velocity from a lifting line model with the uncorrected circulation. In other words, the smearing of the tip vortices in the CFD domain is purposely neglected when the forces on the wings are calculated, which means that the lift-induced drag from the full tip vortices is kept, without any need for additional corrections.

With `maintain_mean_value` set to true, the total projected lift on each wing is the same as the lift computed from the uncorrected circulation. Setting it to false with this option will, therefore, project less lift to the CFD domain than what is acting on the wings, which is generally not what is wanted.

**Note on the limit of the lifting line correction:** with an almost constant circulation, the velocity correction becomes very large at the outermost line elements, and a large part of the lift-induced drag comes from these elements. A low value for the `max_correction_ratio` in the lifting line correction will cut this part of the correction, and therefore also a large part of the lift-induced drag. For rotor sails, a value of 2.0 has been found to work better than 1.0 when the tip loss correction is only applied to the projected forces. This is likely sensitive to the number of line elements, as the correction at the outermost elements depends on how close they are to the end of the wing.
