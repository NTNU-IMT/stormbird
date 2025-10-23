# Simulation overview

The actuator line model is mostly managed through the `ActuatorLine` structure, which is a pure Rust implementation of the **most** of the functionality needed for this medellin type. However, this model cannot execute a complete simulation alone; it needs to be **combined with a CFD solver**, and some of the functionality for the model is also necessary to implement directly in the interface with the CFD solver, rather than in the Stormbird library. In particular, this is the case for of the [velocity sampling](velocity_sampling.md) functionality. See the [CFD interface chapter](cfd_interface.md) for more details on how the coupling works in practice for the case of OpenFOAM.

That being said, as much functionality as possible is still kept on the Stormbird side, to make it easier to use the same library across different CFD solvers.

## Creating a simulation
The builder pattern is used to create an actuator line simulation. The main structure for this is the `ActuatorLineBuilder`, where the available fields is shown below:

```rust
pub struct ActuatorLineBuilder {
    pub line_force_model: LineForceModelBuilder,
    pub projection_settings: ProjectionSettings,
    pub solver_settings: SolverSettings,
    pub sampling_settings: SamplingSettings,
    pub controller: Option<ControllerBuilder>,
    pub write_iterations_full_result: usize,
    pub start_iteration: usize,
    pub lifting_line_correction: Option<LiftingLineCorrectionBuilder>,
    pub empirical_circulation_correction: Option<EmpiricalCirculationCorrection>,
}
```

In the same way as for the rest of the library, this structure can be automatically deserialized from a JSON input file. As such, to create a setup to be used withing a CFD solver, it is mostly a matter of creating the right setup.

All settings have default values, so the only required field to get started is the `line_force_model`. This field specifies the sail geometry, using the exact same format as for lifting line simulations. See the [line model chapter](../line_model/line_model_intro.md) for more details on how to create a line model.

The other settings mainly control how the velocity is sampled from the CFD domain, how the forces are projected back, and what type of corrections to use. There are some details for all of these which can be important to know about. As a **general warning**, actuator line models have more details in the settings, which may also affect the final results more than, for instance, a lifting line simulation. As such, some care should be taken when setting up the model to ensure that the settings are appropriate for the specific case. More on this in the respective chapters for [velocity sampling](velocity_sampling.md), [force projection](force_projection.md) and [corrections](corrections.md).

## Running a simulation
The overall execution of an actuator line simulation is, necessarily, controller from the CFD solver. That is, the duration of the simulation, the time step, what type of disturbances to include, and the general freestream velocity must all be part of the CFD setup. For each time step, the following happens internally in code:

1) The **velocity on the control point of each line segment** in the line force model is sampled from the CFD domain, using one of the available [velocity sampling methods](velocity_sampling.md).
2) **The circulation strength on each line segment is calculated** based on the sampled velocity and the sectional models, in the same way as for the [non-linear solver](../lifting_line/solver.md) in the lifting line model, but with only one iteration. That is, the new circulation is estimated directly from the velocity, but the final values can be based on the previous time step as well, using the same general damping technique as for the lifting line model. The reason for using only one iteration is that no way to update the velocity fields in the CFD domain outside the general solver loop. As such, the number of iterations are inherently controlled by the CFD solver directly, and not by the actuator line model.
3) **If any corrections are enabled**, these are applied to the estimated circulation strength. The corrections may be crucial for accurate results. See the [corrections chapter](corrections.md) for more details on what corrections are available and when they should be used.
4) Based on the estimated circulation strength, the forces on each line segment are calculated, and the **circulatory forces are projected back to the CFD grid** using the methods and settings specified in the [force projection chapter](force_projection.md) chapter.
5) The **[result data](../line_model/force_calculations.md)** for each time step will finally be written to disk, and the actuator line model is ready for the next time step.
