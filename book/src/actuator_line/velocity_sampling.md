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
- `correction_factor`: A global correction factor applied to all sampled velocities, which can be used to either artificially increase or decrease the velocity magnitude. It is set to 1.0 by default.

## Linear interpolation

The first and most direct method is to simply use interpolated velocities from the CFD grid at each control point. This method relies on the built in interpolation methods in the CFD solver. To active this method, simply set the `use_point_sampling` flag to true in the `SamplingSettings`.

## Body force weighted estimate

The second method for velocity sampling is implemented based on the explanation in [this paper](https://www.nrel.gov/docs/fy17osti/67611.pdf). Rather than using a direct interpolation, the velocity at each control point is estimated using a Gaussian weighted average of the velocities in the CFD cells around each control point. The weighting is set to be identical to the kernel used to [project the force](force_projection.md) back into the CFD domain, along the two dimensions normal to the span line.

The additional settings variables for this sampling method are generally connected to how the Gaussian kernel should be along the spanwise direction for each line segment. There are two variables controlling this:

- `span_projection_factor`: This variable sets the width of the Gaussian kernel along the spanwise direction as a multiple of the local line segment length. The default value is 1.0.
- `neglect_span_projection`: If this flag is set to true, the Gaussian kernel is assumed to be infinitely wide along the spanwise direction, effectively reducing the kernel to a 2D Gaussian in the plane normal to the line segment. The default value is false.

This method is claimed to be more robust against numerical noise, in particular when using coarse grids. It also has the benefit that the interpolation method itself is not dependent on the CFD solver so that it is easy to use with any solver.
