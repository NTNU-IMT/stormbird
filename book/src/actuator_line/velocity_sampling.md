# Velocity sampling

As with the lifting line method, the actuator line method needs the velocity at each control point in the line force model to compute the forces acting on each line segment. There are in general two different methods to obtain these, described in the subsections below. The overall structure controlling the sampling method is also shown below:

```rust
pub struct SamplingSettings {
    pub use_point_sampling: bool,
    pub span_projection_factor: Float,
    pub neglect_span_projection: bool,
    pub weight_limit: Float,
    pub extrapolate_end_velocities: bool,
    pub remove_span_velocity: bool,
    pub correction_factor: Float
}
```

Some of the settings are independent of the sampling methods chosen. In particular, this is true for the following settings:

- `extrapolate_end_velocities`: If true, the velocities at the end control points are extrapolated from the inner control points to avoid edge effects. It is set to false by default.
- `remove_span_velocity`: If true, the component of the velocity along the actuator line segment is removed from the sampled velocity. It is set to false by default.
- `correction_factor`: A global correction factor applied to all sampled velocities, which can be used to either artificially increase or decrease the velocity magnitude. It is set to 1.0 by default, which means that it does not change the sampled velocities at all.

## Direct interpolation

The first and most direct method is to simply use interpolated velocities from the CFD grid at each control point. This method relies on the built in interpolation methods in the CFD solver, which means that the order of the interpolation is also chosen by the CFD solver. For a second order unstructured CFD code, like OpenFOAM, the interpolation will often be linear. To active this method, simply set the `use_point_sampling` flag to true in the `SamplingSettings`.

## Body force weighted estimate

The second method for velocity sampling is implemented based on the explanation in the paper by [Churchfield et al., 2017](../literature/simulation_methods.md#an-advanced-actuator-line-method-for-wind-energy-applications-and-beyond-2017). Rather than using a direct interpolation, the velocity at each control point is estimated using a Gaussian weighted average of the velocities in the CFD cells around each control point. The weighting is set to be identical to the kernel used to [project the force](force_projection.md) back into the CFD domain, along the two dimensions normal to the span line.

The additional settings variables for this sampling method are generally connected to how the Gaussian kernel should be along the spanwise direction for each line segment. There are two variables controlling this:

- `span_projection_factor`: This variable sets the width of the Gaussian kernel along the spanwise direction as a multiple of the local line segment length. The default value is 0.5. That is, it is the cells closest to the control point that will influence the sampled velocity the most, also in the spanwise direction. The fall off in the spanwise direction is controller by this factor.
- `neglect_span_projection`: If this flag is set to true, the Gaussian kernel is assumed to be infinitely wide along the spanwise direction, effectively reducing the kernel to a 2D Gaussian in the plane normal to the line segment. However, only cells that are normal to the span line is included in the weighted average; cells that are either *above* or *below* is neglected. The weight of each cell is still based on the 2D force projection kernel, just no longer any spanwise distribution. The default value is false.

This method is claimed to be more robust against numerical noise, in particular when using coarse grids. It also has the benefit that the interpolation method itself is not dependent on the CFD solver so that it is easy to use with any solver.
