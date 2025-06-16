# Simulation overview

The actuator line model is mostly managed through the `ActuatorLine` structure, which can be constructed through the `ActuatorLineBuilder` structure shown below. However, this model cannot execute a complete simulation alone; it needs to be combined with a CFD solver, and some of the functionality for the model is also necessary to implement directly in the interface with the CFD solver, rather than in the Stormbird library. In particular, this is the case for of the [velocity sampling](velocity_sampling.md) functionality. See the [CFD interface chapter](cfd_interface.md) for more on how `ActuatorLine` structure is made to interact with a specific CFD solver (only OpenFOAM at this point).

```rust
pub struct ActuatorLineBuilder {
    pub line_force_model: LineForceModelBuilder,
    pub projection: Projection,
    pub solver_settings: SolverSettings,
    pub sampling_settings: SamplingSettings,
    pub controller: Option<ControllerBuilder>,
    pub write_iterations_full_result: usize,
    pub start_iteration: usize,
    pub extrapolate_end_velocities: bool,
    pub remove_span_velocity: bool
}
```
The purpose of the `ActuatorLine` structure is to managing all the generic functionality that can be thought of as independent of the exact CFD solver. The 