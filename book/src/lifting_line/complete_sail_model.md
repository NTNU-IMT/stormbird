# Complete sail model

A lifting line [`Simulation`](./simulation_overview.md) needs the freestream velocity at every relevant point, and the sail settings, such as the local wing angles and the internal states, must be set from the outside. The `CompleteSailModel` combines a lifting line simulation with a [wind environment](../utils/wind_environment.md) and a [control system](../utils/control_system.md), so that the sails can be simulated as a *complete unit*. The input is then a wind condition and a ship velocity, and the output is the forces and the input power for each sail, with the sails operated as the control system dictates.

The typical use case is to predict the performance of a sail system in many different conditions, for instance in power prediction or voyage simulation tools. The model is available in Rust, in [pystormbird](../different_versions/python_interface.md), and through the [C interface](../different_versions/c_interface.md).

## Input
The model is created from a `CompleteSailModelBuilder`:

```rust
pub struct CompleteSailModelBuilder {
    lifting_line_simulation: SimulationBuilder,
    wind_environment: WindEnvironment,
    controller: ControllerBuilder,
    settings: CompleteSailModelSettings, // Optional
}

pub struct CompleteSailModelSettings {
    pub max_controller_iterations: usize,               // Default: 10
    pub allowed_angle_error: Float,                     // Default: 0.1 degrees, in radians
    pub nr_loadings_to_test_during_optimization: usize, // Default: 10
    pub retractable: bool,                              // Default: false
    pub thrust_direction: SpatialVector,                // Default: (-1, 0, 0)
}
```

- `lifting_line_simulation`: the [simulation builder](./simulation_overview.md) for the sails. Quasi-steady settings are the natural choice for steady state conditions.
- `wind_environment`: the [wind environment](../utils/wind_environment.md), which defines how a wind condition is turned into a freestream velocity at each point, including optional inflow corrections.
- `controller`: the [controller builder](../utils/control_system.md), with one set of set points for each sail.
- `settings`: settings that are specific to the complete sail model, explained in the sections below.

The ship velocity is given as a scalar. The velocity due to the ship motion is the ship velocity times the `zero_direction_vector` in the wind environment, which is added to the true wind to give the apparent wind. In other words, the ship is assumed to sail in the opposite direction of the `zero_direction_vector`, and a wind direction of zero is a head wind. With the default values, the ship sails in the negative x-direction, which is why the default `thrust_direction` is `(-1, 0, 0)`. The [coordinate system](#coordinate-system) section below shows how to use other directions.

## Steady state simulations
There are two main methods for steady state conditions. The Python signatures are shown below. The methods have the same names and arguments in Rust.

```python
def simulate_steady_state_condition(
    self,
    *,
    wind_condition: WindCondition,
    ship_velocity: float,
    controller_loading: float,
) -> SimulationResult
```

```python
def simulate_optimal_steady_state_condition(
    self,
    *,
    wind_condition: WindCondition,
    ship_velocity: float,
    max_loading: float,
) -> SimulationResult
```

`simulate_steady_state_condition` simulates the sails with a given controller *loading*. The loading scales all set points in the controller. A loading of 1.0 gives the set points as specified, while 0.0 gives sails in a neutral position. Values outside this range are clamped. The simulation runs in two steps:

1. The controller is first applied based on the wind condition alone, which neglects the velocities induced by the sails.
2. The model then alternates between running the lifting line simulation and updating the controller based on the result, until the change in the local wing angles is smaller than `allowed_angle_error` or until `max_controller_iterations` is reached. This makes it possible to reach the set points also when they depend on the induced velocities, e.g., when the controller uses the effective angle of attack.

`simulate_optimal_steady_state_condition` runs `simulate_steady_state_condition` for `nr_loadings_to_test_during_optimization` loadings, evenly distributed from 0.0 to `max_loading`, and returns the result with the highest *effective power*:

\\[
    P\_{effective} = \mathbf{F} \cdot \mathbf{e}\_{thrust} \\, V\_{ship} - P\_{input}
\\]

where \\( \mathbf{F} \\) is the sum of the forces on all sails, \\( \mathbf{e}\_{thrust} \\) is the `thrust_direction`, and \\( P\_{input} \\) is the sum of the [input power](../line_model/input_power.md) for all sails. This is mainly useful for sails that need power, such as rotor sails and suction sails. For these sails, the maximum thrust does not necessarily give the best net result. It can also be used to reduce the loading in conditions where the full set points give a drag rather than thrust. If `retractable` is `true`, and the best effective power is still negative, the sails are assumed to be retracted, and the forces, moments and input power in the returned result are set to zero.

### Simple output
The methods above return a full [`SimulationResult`](../line_model/force_calculations.md), which contains sectional values, the input to the force calculations, and more. Usually only the total values for each sail are needed. Both methods therefore also have a `_simple_output` variant, which takes the same arguments but returns a list with one `SingleSailResult` for each sail:

```rust
pub struct SingleSailResult {
    pub force: SpatialVector,  // Total force on the sail
    pub moment: SpatialVector, // Total moment on the sail
    pub input_power: Float,    // Input power to the sail
}
```

The force and moment are the `total` integrated values for each wing in the simulation result. They are given in the output coordinate system of the line force model, which is the global coordinate system by default. In Python, `force` and `moment` are returned as lists with three elements, and `input_power` as a float.

`simulate_optimal_steady_state_condition_simple_output` is likely to be the main way to use the complete sail model. It finds the best operating point in a given condition, and returns only the values that are needed in a typical performance prediction.

## Default sail models
Setting up the lifting line simulation and the controller for a complete sail model requires many choices. The [stormbird_setup](https://github.com/NTNU-IMT/stormbird/tree/main/interfaces/stormbird_setup) library therefore comes with a limited set of default sail models, which can be used to set up a complete sail model with reasonable values for all settings. The models are classified by the `DefaultSail` enum:

| Name | Sail type | Basis |
|---|---|---|
| `GenericSingleElementWingSail` | Single element wing sail | Best guess, based on the default [foil model](../sectional_models/foil_model.md) |
| `GenericTwoElementWingSail` | Wing sail with a flap. The internal state is the flap angle | Best guess |
| `RotorSailDeybach2024` | Rotor sail | Lift fitted to Deybach (2024), *Aerodynamics of the Flettner rotor at high Reynolds number* |
| `RotorSailOstman2023` | Rotor sail | Lift fitted to Ostman (2023), *Calculation of Flettner rotor forces using lifting line and CFD methods* |
| `SuctionSailCharrier1985` | Suction sail. The internal state is the fan power | Tuned to the wind tunnel data for the original TurboSail in Charrier et al. (1985) |

The names indicate whether a model is a generic best guess, or tuned to a specific source. None of the models are intended to represent a specific commercial sail. Each model is a `SailDefinition` that contains:

- The [section model](../sectional_models/sectional_models_intro.md), which also gives the [input power](../line_model/input_power.md).
- The [controller set points](../utils/control_system.md). The angle of attack set points are *effective* angles of attack, so that multiple sails are operated close to the set points, also when they affect each other. The maximum angle of attack is 5 degrees below the angle where the 2D lift is at its maximum.
- Simulation settings, such as the [solver](./solver.md) and the [circulation correction](../line_model/circulation_strength.md), together with optional virtual extensions of the span.

A definition does not contain the size of the sail, and can be scaled to any chord length (diameter for rotor sails) and height. Limits that depend on the size, such as the maximum revolutions per second of a rotor sail, are scaled automatically.

Each model is available for both the linearized and the iterative [solver](./solver.md), selected with the `SolverType` enum. The linearized solver is the default. The two versions only differ in the stall parameters of the foils, which are set so that the maximum lift, and the angle of attack where it occurs, are about the same for both solvers.

### Setting up a model
The `CompleteSailModelBuilder.new_from_default_sail` class method sets up a complete sail model with a sail at each of the given positions:

```python
from stormbird_setup.default_sails import DefaultSail, SolverType
from stormbird_setup.lifting_line import CompleteSailModelBuilder, SymmetryCondition
from stormbird_setup.spatial_vector import SpatialVector

builder = CompleteSailModelBuilder.new_from_default_sail(
    sail = DefaultSail.RotorSailDeybach2024,
    positions = [SpatialVector(x = -40.0, z = 10.0), SpatialVector(x = 40.0, z = 10.0)],
    chord_length = 5.0, # The diameter, for rotor sails
    height = 35.0,
    symmetry_condition = SymmetryCondition.Z,
    solver_type = SolverType.Linearized, # Optional, linearized by default
)

# Input to, e.g., the complete sail model in pystormbird or cstormbird
builder.to_json_file("sail_model.json")
```

- `positions`: the bottom of each sail. All sails use the same model and size.
- `symmetry_condition`: the symmetry condition of the [wake](./wake.md). With `SymmetryCondition.Z`, the plane at z = 0 represents the water surface. A sail with the base on the plane continues as a mirror image below it, which is detected from the position. A sail above the plane, as on the deck of a ship, gets a gap between the base and its mirror image.
- `wind_environment` and `settings`: optional, see the [coordinate system](#coordinate-system) section below.

The `sail` can also be a custom `SailDefinition`, for instance a modified default sail. As the definition is a normal stormbird_setup class, it can be stored as a JSON file, and read again later:

```python
from stormbird_setup.default_sails import SailDefinition

definition = DefaultSail.SuctionSailCharrier1985.definition()
definition.virtual_extension_factors = (0.0, 0.5)

definition.to_json_file("my_sail.json")

definition = SailDefinition.from_json_file("my_sail.json")
```

### Coordinate system
The orientation of the sails follows the [wind environment](../utils/wind_environment.md):

- The span of each sail goes along the `up_direction`.
- The chord goes along the `zero_direction_vector`. The ship sails in the opposite direction, so a local wing angle of zero aligns the sails with a head wind.
- The `thrust_direction` in the settings is set opposite of the `zero_direction_vector`, i.e., in the sailing direction.

The default wind environment has the x-axis pointing backwards and the z-axis pointing up, so that the ship sails in the negative x-direction. To use another coordinate system, give a wind environment with different directions. The `wind_rotation_axis` should then be opposite of the up direction, so that the wind direction is measured in the same way, i.e., clockwise seen from above. For instance, for a coordinate system with the x-axis pointing forward, the y-axis to starboard and the z-axis pointing down:

```python
from stormbird_setup.wind import WindEnvironment

wind_environment = WindEnvironment(
    up_direction = SpatialVector(z = -1.0),
    wind_rotation_axis = SpatialVector(z = 1.0),
    zero_direction_vector = SpatialVector(x = -1.0), # The ship sails in the positive x-direction
)

builder = CompleteSailModelBuilder.new_from_default_sail(
    sail = DefaultSail.RotorSailDeybach2024,
    positions = [SpatialVector(x = 40.0, z = -10.0), SpatialVector(x = -40.0, z = -10.0)],
    chord_length = 5.0,
    height = 35.0,
    symmetry_condition = SymmetryCondition.Z,
    wind_environment = wind_environment,
)
```

The sails then extend in the negative z-direction from the deck, which is 10 m above the water surface, and the thrust direction is `(1, 0, 0)`. With both coordinate systems, a wind direction of 90 degrees is a wind from starboard, and the results are the same, apart from the directions of the force vectors. Note that the controller currently uses the opposite sign for the apparent wind direction, as explained in the [control system](../utils/control_system.md#sign-of-the-apparent-wind-direction) chapter.

**Note**: if the `settings` argument is given, the settings are used as they are, and the thrust direction is not set from the wind environment. It must then be set manually, if it is not the default `(-1, 0, 0)`:

```python
from stormbird_setup.lifting_line.complete_sail_model import CompleteSailModelSettings

settings = CompleteSailModelSettings(
    retractable = True,
    thrust_direction = SpatialVector(x = 1.0),
)
```

## Example
The example below runs the rotor sail model from above with pystormbird, for a range of wind directions, using `simulate_optimal_steady_state_condition_simple_output`:

```python
import numpy as np

from stormbird_setup.default_sails import DefaultSail
from stormbird_setup.lifting_line import CompleteSailModelBuilder, SymmetryCondition
from stormbird_setup.spatial_vector import SpatialVector

from pystormbird.lifting_line import CompleteSailModel
from pystormbird.wind import WindCondition

builder = CompleteSailModelBuilder.new_from_default_sail(
    sail = DefaultSail.RotorSailDeybach2024,
    positions = [SpatialVector(x = -40.0, z = 10.0), SpatialVector(x = 40.0, z = 10.0)],
    chord_length = 5.0,
    height = 35.0,
    symmetry_condition = SymmetryCondition.Z,
)

model = CompleteSailModel(builder.to_json_string())

ship_velocity = 12.0 * 0.5144  # 12 knots in m/s

for wind_direction_deg in range(0, 181, 30):
    wind_condition = WindCondition.new_constant(
        direction_coming_from = np.radians(wind_direction_deg),
        velocity = 8.0
    )

    results = model.simulate_optimal_steady_state_condition_simple_output(
        wind_condition = wind_condition,
        ship_velocity = ship_velocity,
        max_loading = 1.0
    )

    thrust = sum(-result.force[0] for result in results)
    input_power = sum(result.input_power for result in results)

    print(
        f"Wind direction: {wind_direction_deg:3d} deg, "
        f"thrust: {thrust / 1000:6.1f} kN, "
        f"input power: {input_power / 1000:6.1f} kW"
    )
```

The thrust is the negative x-component of the force, which matches the default `thrust_direction`. An example that compares all the default sail models can be found in the [pystormbird examples folder](https://github.com/NTNU-IMT/stormbird/tree/main/interfaces/pystormbird/examples).

## Other methods
The model also has methods for more direct control of the simulation:

- `do_step` and `do_multiple_steps`: run the lifting line simulation for a given wind condition and ship velocity, with the current sail settings. The controller is **not** applied. `do_multiple_steps` re-initializes the wake and runs from time zero to `end_time`, and is mainly useful with dynamic simulation settings, for instance with [gusts](../utils/wind_environment.md) in the wind condition.
- `apply_controller_based_on_wind_condition`, `apply_controller_based_on_freestream` and `apply_controller_based_on_simulation_result`: update the sail settings with the controller, based on different types of input.
- `set_local_wing_angles` and `set_internal_state`: set the sail settings manually. Any later controller update overwrites them. In Python, there are also `set_internal_state_linear` and `set_internal_state_piecewise_constant` for internal states that vary along the span.
- `freestream_velocity`: the apparent wind at all the points where the simulation needs it, including the ship velocity and any inflow corrections.
- `local_wing_angles`, `internal_state`, `get_number_of_sails` and `get_wind_environment` (Python): query the current state of the model.
