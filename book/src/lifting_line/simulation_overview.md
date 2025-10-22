# Simulation overview

Lifting line simulations in Stormbird are managed and executed through a specialized `Simulation` structure. The responsibility of this structure is to store and update the data necessary for a simulation. It can be executed once, for steady-state conditions, or for many time steps, in dynamic conditions. When executed many times, the results from the previous time steps are used as initial conditions for the next time steps.

## Creating a simulation

To construct a `Simulation`, a `SimulationBuilder` is used. An overview of the fields in the builder is shown below:

```rust
pub struct SimulationBuilder {
    pub line_force_model: LineForceModelBuilder,
    pub simulation_settings: SimulationSettings,
}
```

The only input that is absolutely necessary to specify is the [builder for a line force model](./../line_model/building_line_model.md). The simulation settings structure have default variables.

Simulations in Python are created through a `Simulation` class that takes a JSON string containing the  data for the `SimulationBuilder`.

```python
from pystormbird.lifting_line import Simulation
import json

# Some code to generate setup string before this for both the line force model
# and the simulation settings.

setup_dict = {
    "line_force_model": line_force_model_dict,
    "simulation_settings": simulation_settings_dict
}

simulation = Simulation(
    setup_string = json.dumps(setup_dict)
)
```

## Simulation settings
The simulation settings is an Enum that specifies whether the simulation should be executed using the quasi-steady or the dynamic variant of the lifting line. Each variant includes its own settings, which gives the necessary input to each method. The point of collecting both methods into the same structure is to generate an interface where the same line force model can easily be executed in the same way using both methods. This is, for instance useful for comparison cases.

The Enum looks like this:

```rust
pub enum SimulationSettings {
    QuasiSteady(QuasiSteadySettings),
    Dynamic(DynamicSettings),
}
```

Both the `QuasiSteadySettings` and the `DynamicSettings` have the same general fields: one structure for the [solver](./solver.md) and another for the [wake](./wake.md). The actual rust definition looks like this:

```rust
pub struct QuasiSteadySettings {
    pub solver: QuasiSteadySolverBuilder,
    pub wake: QuasiSteadyWakeSettings,
}

pub struct DynamicSettings {
    pub solver: Solver,
    pub wake: DynamicWakeBuilder,
}
```

## Running a simulation
Executing a simulation after a `Simulation` structure is made is done with a function called `do_step`. On the Rust side, it has the following signature:

```rust
pub fn do_step(
    &mut self,
    time: f64,
    time_step: f64,
    freestream_velocity: &[SpatialVector]
) -> SimulationResult
```

The input is the current time, time step, and an a vector containing the freestream velocity at all relevant points for the model. See the [velocity input section](./velocity_input.md) for more on how this vector is defined and how to generate it.

On the Python side, the same function looks like this [^note1]:

```python
def do_step(
    self,
    *,
    time: float,
    time_step: float,
    freestream_velocity: list[list[float]],
) -> SimulationResult
```

That is, the python code takes in the same input as the Rust side, but with the equivalent Python data structures. The `SpatialVector` input is actually just a wrapper around an array with three elements, representing the velocity components in x, y, and z orientation. On the Python side, one can pass in a list with many three-elements sub-lists that will be converted to `SpatialVectors` inside the Python wrapper function before being passed to the Rust code.

If the simulation is executed using the quasi-steady approach, the time step will not generally affect the results [^note2]. That means that a steady simulation can be executed by running a quasi-steady simulation only once.

The return from each time step is a [SimulationResult](./../line_model/force_calculations.md). This structure has a Python implementation as well, with some minor helper methods to interpret the results.

[^note1]: The actual implementation is actually written slightly different as it is written in Rust and uses PyO3 to generate the Python interface. However, the code shown represents how it would have look like if it were written as Python code directly.

[^note2]: This is only true for the first time step. It will never affect the circulatory lift, but it may add forces from added mass effects and dynamic rotation effects on the foil, if these effects are turned on. They are not turned on by default, though. In addition, these effects are always turned off for the first time step, as no motion history is available. That is, the acceleration and translation and rotation velocity is always assumed to be zero at the first time step.
