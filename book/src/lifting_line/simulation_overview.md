# Simulation overview

Lifting line simulations in Stormbird are managed and executed through a specialized `Simulation` structure. The responsibility of this structure is to store and update the data necessary for a simulation. It can be executed once, for steady-state conditions, or for many time steps, in dynamic conditions. When executed many times, the results from the previous time steps are used as initial conditions for the next time steps. 

To construct a `Simulation`, a `SimulationBuilder` is used. An overview of the fields in the builder is shown below:

```rust
pub struct SimulationBuilder {
    pub line_force_model: LineForceModelBuilder,
    pub simulation_mode: SimulationMode,
    pub write_wake_data_to_file: bool,
    pub wake_files_folder_path: String,
}
```

The only input that is absolutely necessary to specify is the [builder for a line force model](./../line_model/building_line_model.md). The other variables have default settings. The fields `write_wake_data_to_file` and `wake_files_folder_path` are only used in dynamic simulations at the moment, but are intended to also be implemented for quasi-steady simulations at a later stage. They control whether wake data should be exported to `.vtp` files during a simulation. The point of this is to allow the wake shape and strength to be visualized, which is useful for debugging purposes, or to make illustrations of the simulation process.

## Simulation mode
The simulation mode is an Enum that specifies whether the simulation should be executed using the quasi-steady or the dynamic variant of the lifting line. Each variant includes its own settings, which gives the necessary input to each method. The point of collecting both methods into the same structure is to generate an interface where the same line force model can easily be executed in the same way using both methods. This is, for instance useful for comparison cases. 

The Enum looks like this:

```rust
pub enum SimulationMode {
    QuasiSteady(SteadySettings),
    Dynamic(UnsteadySettings),
}
```

Both the `SteadySettings` and the `DynamicSettings` have the same general fields: one structure for the [solver settings](./solver_settings.md) and another for building the wake []. The actual rust definition looks like this:

```rust
pub struct SteadySettings {
    pub solver: SteadySolverSettings,
    pub wake: SteadyWakeBuilder,
}

pub struct UnsteadySettings {
    pub solver: UnsteadySolverSettings,
    pub wake: UnsteadyWakeBuilder,
}
```

## Running a simulation
Executing a simulation after a `Simulation` structure is made is done with a function called `do_step`. On the Rust side, it has the following signature: 

```rust
pub fn do_step(
    &mut self, 
    time: f64,
    time_step: f64,
    input_state: InputState
) -> SimulationResult
```

The input is the current time, time step, and an "input state" structure which contains information about the current position and rotation of the wings, as well as the [freestream velocity](./velocity_input.md).

On the Python side, the same function looks like this [^note1]:

```python
def do_step(
    self,
    *,
    time: float, 
    time_step: float,
    freestream_velocity: Vec3,
    translation: Vec3,
    rotation: Vec3,
) -> SimulationResult
```

That is, the python code takes in velocity [^note2], translation and rotation as input, rather than a separate input structure. As a general rule, functions should not have many input arguments with the same type, as this makes it easy to screw up the order of the input (for instance, switching rotation with velocity). However, Python supports named function arguments, and also forcing the user to use named functions arguments. Longer function signatures are therefore accepted in Python, even if it is not in Rust.

The `Vec3` input has both a Rust implementation and a Python implementation. It is a three-dimensional vector with x, y, and z fields, which can represent anything a vector would represent (e.g., position, rotation, velocity, force, moments, etc.)

If the simulation is executed using the quasi-steady approach, the time step will not generally affect the results [^note3]. That means that a steady simulation can be executed by running a quasi-steady simulation only once.

The return from each time step is a [SimulationResult](./../line_model/results.md). This structure has a Python implementation as well, with some minor helper methods to interpret the results. 

[^note1]: The actual implementation is actually written slightly different as it is written in Rust and uses PyO3 to generate the Python interface. However, the code shown represents how it would have look like if it were written as Python code directly.

[^note2]: The Python version currently has limited support for different velocity inputs, even though the Rust implementation does support more variants. Synchronizing this is high on the todo list and more functionality on the Python side will come soon.

[^note3]: This is only true for the first time step. It will never affect the circulatory lift, but it may add forces from added mass effects and dynamic rotation effects on the foil, if these effects are turned on. They are not turned on by default, though. IN addition, these effects are always turned off for the first time step, as no motion history is available. That is, the acceleration and translation and rotation velocity is always assumed to be zero at the first time step.

