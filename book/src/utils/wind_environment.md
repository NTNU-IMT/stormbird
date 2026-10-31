# Wind environment

The Stormbird library contains a data structure for representing the wind environment in different situations. The main point of this structure is to be able to provide spatially varying inflow conditions at all relevant points in a simulation. This fits most naturally with the lifting line simulations, but the wind model is kept separate so that it can, potentially, also be used with other simplified models of wind propulsion devices if needed.

The basic structure for the wind environment is given below:

```rust
pub struct WindEnvironment {
    pub up_direction: SpatialVector,
    pub wind_rotation_axis: SpatialVector,
    pub zero_direction_vector: SpatialVector,
    pub water_plane_height: f64,
    pub inflow_corrections: Option<InflowCorrections>,
}
```

The fields in the structure are described below:

- `up_direction`: A vector defining the "up" direction in the simulation. This is used to determine the height above the water surface.
- `wind_rotation_axis`: A vector defining the axis where the wind direction is rotated around when changing wind direction. The default is the negative z-axis, i.e., opposite of the default up direction, so that the wind direction is measured clockwise seen from above.
- `zero_direction_vector`: A vector defining the reference direction for the wind. The wind will point along this vector when the wind direction angle is zero.
- `water_plane_height`: A scalar defining the height of the water plane in the simulation.
- `inflow_corrections`: An optional structure containing corrections to be applied to the inflow velocity, which is primarily intended for modeling disturbances due to the rest of the ship. See more about this correction model below.

## Height variation models
The wind environment only contains the settings that are assumed to be constant for a given model. The variables that describe the true wind at a given instance, including how the wind speed varies with height above the water surface, are instead given as input through a separate `WindCondition` structure:

```rust
pub struct WindCondition {
    pub direction_coming_from: f64,
    pub velocity_variation: VelocityVariation,
    pub parallel_gust: Option<DiscretizedSpectrum>,
    pub perpendicular_gust: Option<DiscretizedSpectrum>,
    pub vertical_gust: Option<DiscretizedSpectrum>
}
```

The height variation models can be set with the following Enum:

```rust
pub enum VelocityVariation {
    Constant(f64),
    PowerModel(PowerModel),
    LogarithmicModel(LogarithmicModel),
}
```

That is, one can choose between a constant wind speed at all heights, a power law model, and a logarithmic model for the wind speed variation with height. The input structures for the two height-dependent models are shown below:

```rust
pub struct PowerModel {
    pub reference_velocity: f64,
    pub reference_height: f64,
    pub power_factor: f64,
}

pub struct LogarithmicModel {
    pub friction_velocity: f64,
    pub surface_roughness: f64,
    pub obukhov_length: Option<f64>,
    pub von_karman_constant: f64,
    pub stable_coefficient: f64,
    pub unstable_coefficient: f64
}
```

For the power model, the `reference_height` is set to 10 m by default and the `power_factor` to 1/9. For the logarithmic model, the `friction_velocity` and `surface_roughness` must be specified. The `obukhov_length` is optional, and can be used to correct the profile for non-neutral (stable or unstable) atmospheric conditions. The remaining fields have default values that should be useful in most cases.

## Inflow corrections
The `inflow_corrections` variable is a data structure used to store corrections to the freestream velocity due to anything that might disturb the inflow. That is, it is primarily intended to model the wind disturbances caused by the ship itself. The data structure is set up such that the correction can depend on both the spanwise position along each sail and the apparent wind direction.

The correction factors themselves consist of two parts: A magnitude factor that scales the local inflow velocity, and an angle correction, in radians, that rotates the local inflow velocity around the `up_direction`.

The data structures to set this up are shown below. In practice, these structures are just used to do linear interpolation on the correction factors as needed during the simulation:

```rust
pub struct InflowCorrectionsSingleDirection {
    pub height_values: Vec<Float>,
    pub magnitude_corrections: Vec<Float>,
    pub angle_corrections: Vec<Float>, 
    pub wing_indices: Vec<Range<usize>>,
}

pub struct InflowCorrections {
    pub apparent_wind_directions: Vec<Float>,
    pub corrections: Vec<InflowCorrectionsSingleDirection>,
}
```

The corrections are applied as follows:

- **Each sail has its own corrections.** The `wing_indices` give the range of values in the vectors that belong to each sail. The corrections for a sail are interpolated linearly between the `height_values`, where the height of a control point is its position along the `up_direction`. A single value for a sail gives a correction that is constant along the span.
- **The corrections are interpolated between apparent wind directions.** The corrected velocity is computed with the corrections for each of the `apparent_wind_directions`, and then interpolated linearly at the current apparent wind direction. The apparent wind direction is measured from the `zero_direction_vector`, around the `wind_rotation_axis`. In the complete sail model, it is computed at a reference height of 10 m.
- **The directions must be sorted.** The `apparent_wind_directions` must be in increasing order. Outside the range of the given directions, the closest corrections are used. There is no wrap-around at ±180 degrees, so the corrections must cover the full range of directions that are relevant.

### Tuning the inflow corrections
The correction factors must come from an external source, typically CFD simulations of the ship. There are two general approaches to find them.

#### Approach 1: velocity measurements without sails
The first approach is to simulate the ship without any sails, and to measure the velocity at the positions where the sails will be placed. The magnitude correction is then the ratio between the measured velocity and the undisturbed apparent wind at the same location, and the angle correction is the angle between the two. This gives corrections that vary along the span, which is the main advantage of the approach. However, it does not capture any interaction between the sails and the flow around the ship, and the corrections are only as good as the assumption that the velocity without sails is the relevant inflow to the sails.

One way to do the measurements is to use an actuator line model in the CFD simulation, where the sails use the `EffectiveWindSensor` section model. The sails then give no forces, but the velocity is still sampled at the control points. The `InflowCorrectionsSingleDirection.from_simulation_result` function in `stormbird_setup` computes the corrections from the result of such a simulation:

```python
import json

from stormbird_setup.wind import (
    WindEnvironment, WindCondition, PowerModel, InflowCorrectionsSingleDirection
)

wind_environment = WindEnvironment()

# Must match the inflow conditions in the CFD simulation, including any variation with height
wind_condition = WindCondition(
    direction_coming_from = direction,
    velocity_variation = PowerModel(reference_velocity = 10.0)
)

with open("postProcessing/stormbird_full_results/full_results_1000.json") as file:
    stormbird_result = json.load(file)

corrections = InflowCorrectionsSingleDirection.from_simulation_result(
    stormbird_result,
    wind_environment,
    wind_condition,
    ship_velocity = 5.0
)

apparent_wind_direction = wind_environment.apparent_wind_direction_from_condition_and_linear_velocity(
    wind_condition,
    wind_environment.linear_velocity_from_ship_velocity(5.0)
)
```

The undisturbed apparent wind is computed from the wind environment, the wind condition, and the ship velocity, which is along the `zero_direction_vector`, as in the complete sail model. Only the part of the measured velocity that is normal to the `up_direction` is used. The sails do not need to be aligned with the flow.

#### Approach 2: force tuning
The second, and generally more accurate, approach is to tune the corrections so that the forces from a lifting line simulation match force measurements, for instance from CFD simulations with the sails present. The measurements can either come from simulations with the full geometry of the sails, or from simulations with actuator lines. This includes the interaction between the sails and the flow around the ship, but gives only one magnitude correction and one angle correction for each sail and condition, which are constant along the span.

The choice of measurement affects what the corrections represent:

- **Actuator line simulations** use the same section models as the lifting line simulation. Any difference in the forces can then be attributed to the inflow, so the corrections isolate the effect of the ship.
- **Simulations with the full geometry** also include any errors in the section models, which the corrections will then compensate for as well. That can be useful, but means that the corrections are tied to the section models used in the tuning.

In both cases, the measured forces must be the total force on each sail, in the same coordinate system as the lifting line simulation, and the sails must have the same settings as in the measurements, e.g., the same local wing angles and internal states.

The tuning based on forces can be done with the `InflowCorrectionsForceTuner` class in `stormbird_setup`. It runs lifting line simulations, and therefore requires pystormbird, which is an optional dependency that can be installed with `pip install stormbird_setup[force-tuning]`. The tuner takes a builder for a quasi-steady lifting line simulation and a wind environment. Each measurement is given as a `ForceTuningCase`:

```python
from stormbird_setup.wind import (
    WindEnvironment, WindCondition, PowerModel, InflowCorrectionsForceTuner, ForceTuningCase
)

tuner = InflowCorrectionsForceTuner(
    simulation_builder = simulation_builder, # Must use quasi-steady settings
    wind_environment = WindEnvironment()
)

cases = [
    ForceTuningCase(
        wind_condition = WindCondition(
            direction_coming_from = direction,
            velocity_variation = PowerModel(reference_velocity = 10.0)
        ),
        ship_velocity = 5.0,
        local_wing_angles = local_wing_angles,      # One value for each sail
        target_forces = measured_forces,            # One SpatialVector for each sail
    )
    for direction, local_wing_angles, measured_forces in measurements
]

result = tuner.tune(cases)

wind_environment = WindEnvironment(inflow_corrections = result.inflow_corrections)
```

For each case, the magnitude and angle corrections for all sails are found together, as the sails affect each other through the induced velocities. This is done with a least squares optimization of the difference between the simulated and the measured forces, relative to the mean magnitude of the measured forces in the case. The corrections are combined into inflow corrections that are sorted by the apparent wind direction, which is computed at the `reference_height` in the settings, 10 m by default, as in the complete sail model. The cases must therefore have different apparent wind directions. To tune corrections for different ship velocities, tune the cases for each velocity separately.

The `ForceTuningSettings` control the bounds of the corrections, optional weights for the error in each force component, the reference height, and the step for the finite difference approximation in the optimization. The result for each case contains the tuned corrections, the simulated forces, the remaining error, and whether the optimization converged. This should be checked, as corrections that end at the bounds indicate that the forces can not be matched with a reasonable correction of the inflow alone.
