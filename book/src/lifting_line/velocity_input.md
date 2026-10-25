# Velocity input

As shown in the [simulation overview section](./simulation_overview.md), to simulate a single time step using the lifting line methods, it is necessary to call the `do_step` function with a vector (in Rust) or list (in Python) of three dimensional spatial vectors as input. This input is labeled the `freestream_velocity`, and represents the freestream velocity at all **relevant points** in the lifting line simulation.

Which points this is depends on the type of simulation. For the quasi-steady cases, it is only the control points of the line force model, as the wake downstream of the wings are not affected by local velocities. For the dynamic simulation it is both the control points and the wake points, as the wake shape is integrated from the velocity field.

## Why spatially varying input?
The point of specifying the velocity at each of these points individually is that this opens up for models that supply spatially varying velocity fields to the simulations. This can for instance be used to incorporate the following things in the simulation:

- **Atmospheric boundary layer model:** The wind speed - and potentially also direction - will vary depending on the height above the ocean. A simplified model of the atmospheric boundary layer can be used to generate different velocities for each relevant point.
- **Simplified models of viscous wakes:** The flow field on a ship will often be affected by separated flow from various superstructures and deck equipment. The flow might also be affected by separated flow from other sails. There exist simplified models to account for this. These models can be connected to the lifting line simulations by affecting the input freestream velocity. In that case, the position of each point in the simulation matters for how the velocity should be affected.
- **CFD data as input:** Using CFD data is a possible way to account for interactions with the rest of the ship. In that case, a velocity field from a simulation of the deck and superstructure can be used to generate an interpolation model, which later is used to specify a spatially varying velocity fields as input to the lifting line.

## Code example

Generating the right velocity input consists of two steps. First, the user must query the simulation model for the relevant points. This happens by calling the `get_freestream_velocity_points` method. The `Simulation` structure/class will then return the right points, which as already mentioned, depends on the method used. These points can then be processed by the user to generate a vector/list of spatial vectors for each of the relevant points, which are later given as input to the `do_step` method. A slightly simplified example is shown below. See the [code examples](https://github.com/NTNU-IMT/stormbird/tree/main/interfaces/pystormbird/examples) for more.

```python
from pystormbird.lifting_line import Simulation

# ----- code to set up the simulation first -----
simulation = Simulation(setup_string)

# Assumes this is a class in Python that models variation in wind speed as a function of height
wind_model = AtmosphericBoundaryLayer(
    ship_velocity           = ship_velocity,
    reference_wind_velocity = wind_velocity,
    wind_direction          = wind_direction
)

# Query the simulation model for the right points
freestream_velocity_points = simulation.get_freestream_velocity_points()

# Generate velocity vectors for each point
freestream_velocity = []
for point in freestream_velocity_points:
    local_velocity = wind_model.get_velocity(point) # List, such as [u_x, u_y, u_z]

    freestream_velocity.append(local_velocity)

# Run the simulation with the generated freestream velocity input
current_time = 0.0
time_step = 0.1

while current_time < end_time:
    result = simulation.do_step(
        time = current_time,
        time_step = time_step,
        freestream_velocity = freestream_velocity
    )

    current_time += time_step
```
