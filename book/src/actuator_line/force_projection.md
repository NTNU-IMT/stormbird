# Force projection

The forces calculated on each line segment need to be projected back onto the CFD grid as body force to deflect the flow in as similar way as possible to a real wing. In other words, the force projection step is a requirement for actually introducing lift-induced velocities in the simulation. Without it, the force on each sail will not be affected by neither itself or other sails in the simulation. The force projection step is therefore a crucial part of the actuator line method.

There are a few different ways to project these forces, and the choice of method can have a significant effect on the final results.

## Basic principle

For each line segment in the line force model, there are force vectors representing different [force types](../line_model/force_calculations.md). When running an actuator line simulation, it is only the **circulatory force** that is projected by default. The other force types are still modeled, but not projected back to the CFD grid.

The force on each line segment is assumed to act at the control points, or the middle of each line segment. However, it is not possible to simply add a single force vector for each point back to the CFD domain. If, lets say, each force where added to the cell closest to each control point, the resulting force field would be very extreme, and cause instabilities in the flow domain. As such, the force on each line segment needs to be distributed over a volume around each control point.

The goal with the force projection logic, then, is to find a way to distribute the calculated forces on each line segment in a way that is both **smooth** AND than creates a **flow deflection that is as close as possible to the one created by a real wing**.

## Background for the choice of force projection method

The method implemented in Stormbird is based on a combination of techniques from the literature. It can be summarized as a two-dimensional Gaussian kernel in the plane normal to each line segment, but with anisotropic widths in the chord direction and the direction normal to both the chord and span directions. The last direction is also referred to as the "thickness direction".

Below is a short summary of how three different papers have influenced the final choice of method.

### Original method
The original, and still most common, method for force projection, as for instance explain in the paper by [Sørensen et. al (2002)](../literature/simulation_methods.md#numerical-modeling-of-wind-turbine-wakes-2002), uses a uniform Gaussian kernel. That is, the width of the kernel is identical in all directions. This creates a volumetric distribution that is smooth everywhere and easy to implement. As such, it is still a common choice in many papers describing actuator line methods.

However, this method was thought to have a few practical issues. The biggest one is that it may result in forces being projected significantly far outside the tip and root regions of the wing, as the width of the Gaussian kernel is, necessarily, not directly dependent on the length of the line segment. This creates situations where the width of the Gaussian kernel should really be tuned very carefully to the number of line segments in the model, or one risks having an effective span length that is larger than the real wing. In addition, it might create non-physical circulation distribution if the Gaussian kernel width is set too small, compared to length of the line segment.

### Introduction of the two-dimensional kernel
An simple solution to the issues described above was introduced in the PhD-thesis by [Mikkelsen, 2004](../literature/simulation_methods.md#actuator-disc-methods-applied-to-wind-turbines-2004). Rather than using the simple uniform kernel, he suggested a 2D kernel, where the force is smoothed only in the directions normal to the line segment, while being kept constant in strength along the spanwise direction of each line segment. Outside the line segment, the volume force is set to zero. This allows the force projection width to be set independently of the length of each line segment. The resulting force distribution is, also, more similar to how it would be in a discrete lifting line method with constant strength vortex elements.

**Stormbird therefore uses a two-dimensional Gaussian kernel for the force projection**, rather than the conventional 3D kernel.

### Anisotropic kernel

A final improvement available in the Stormbird library, relative to the original method, was inspired by the suggested force distribution in the paper by [Churchfield et al., 2017](../literature/simulation_methods.md#an-advanced-actuator-line-method-for-wind-energy-applications-and-beyond-2017). They tested an anisotropic Gaussian distribution, where the width of the kernel could be set independently in the chordwise, thickness, and spanwise directions. The idea was that most wings have a thickness that is significantly smaller than the chord length. By distributing the force more narrowly in the thickness direction, the resulting flow deflection could, perhaps, be more similar to a real wing. They show results that indicate that an anisotropic kernel can provide better results in terms of measured lift and drag forces than an isotropic one.

**Stormbird therefore allows for setting different widths in the chord and thickness directions**. The distribution in the spanwise direction still follows the same logic as for the unfiform two-dimensional kernel suggested by [Mikkelsen, 2004](../literature/simulation_methods.md#actuator-disc-methods-applied-to-wind-turbines-2004).

## Input structures

The force distribution in Stormbird is controlled through the `ProjectionSettings` and `Gaussian` structures, shown below:

```rust
pub struct Gaussian {
    pub chord_factor: f64,
    pub thickness_factor: f64,
}

pub struct ProjectionSettings {
    pub projection_function: Gaussian,
    pub project_normal_to_velocity: bool,
    pub weight_limit: f64,
    pub project_sectional_drag: bool,
}
```

The `Gaussian` structure controls the width of the Gaussian kernel in the chord and thickness directions. The width in each direction is set as a multiple of the local chord length. For instance, if the `chord_factor` is set to 0.25, the width of the Gaussian kernel in the chord direction will be equal to a quarter of the chord length.

The `ProjectionSettings` structure contains some additional settings for the force projection step. The most important ones are described below:

- `project_normal_to_velocity`: If true, the force vector on each line segment is projected onto the plane normal to the local velocity vector before being distributed to the CFD grid. This is mostly a feature implemented for testing purposes. It is set to false as default, which is also the recommended setting.
- `weight_limit`: This variable sets a lower limit for the weight of each CFD cell in the force projection step. Cells with a weight lower than this limit will not receive any force contribution from the line segment. This is mainly used in the CFD interface to determine which cells that needs to be looped over or not during the force projection step. The default value is 0.001.
- `project_sectional_drag`: If true, the sectional drag force on each line segment is also projected back to the CFD grid, in addition to the circulatory force. It is set to false as default, which is also the recommended setting.
