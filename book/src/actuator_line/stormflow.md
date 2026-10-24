# Stormflow

[Stormflow](https://github.com/NTNU-IMT/stormbird/tree/main/stormflow) is a CFD solver made specifically for actuator line simulations with Stormbird. It is not a general CFD solver. It only has the functionality needed to simulate sails, and the effect that other structures, such as the ship, have on the flow to the sails. The idea is that a simple, dedicated solver can be a good fit for this type of modeling, as the actuator line method does not need to resolve boundary layers. The solver is still experimental.

The main way to use Stormflow is through the Python interface, `pystormflow`, with the input made by the [stormbird_setup](./scripted_setup.md) library.

## Method

### Flow solver
Stormflow solves the incompressible Navier-Stokes equations on a cartesian grid where all cells have the same size. The equations are discretized with fourth order finite differences, and solved with explicit time stepping and a pressure projection step. The pressure is solved with a geometric multigrid method.

A uniform grid needs more cells than a mesh that is refined locally, so the solver is written with speed as a priority. Both the velocity solver and the pressure solver can run either on the CPU or on the GPU, and the choice is made separately for each. Running both on the GPU is the fastest. The actuator line model itself always runs on the CPU, but only the velocity and the forces in the cells close to the wings are transferred between the CPU and the GPU.

### Actuator line
The actuator line model is the same as described in the rest of this chapter: the velocity is sampled from the flow field, the forces are computed with the sectional models in Stormbird, and the forces are projected back to the grid as body forces.

### Geometries
Other structures can be included as triangle meshes (`.obj` files), cuboids, spheres or disks. They are represented with an immersed boundary method on the cartesian grid, in a deliberately simplified way. The goal is not to predict the boundary layer on the structures, but to capture their overall effect on the flow to the sails.

- **Slip geometries** (`slip_geometries`) only stop the flow from going through the surface.
- **No-slip geometries** (`geometries`) also remove momentum from the flow. By default, this is done with a *wall model*, which adds the wall shear stress from the log-law close to the surface. The alternative is the *data immersion* method, which blends the velocity towards zero in a thin band around the surface. The data immersion is more robust for thin geometries, but the friction then depends on the grid resolution. The wall model requires geometries that are at least a few cells thick.

Since the boundary layers are not resolved, nothing would force the flow to separate at sharp edges on a coarse grid. The velocity is therefore damped close to the sharp convex edges of the geometries, which fixes the separation points at the edges. This is on by default for both slip and no-slip geometries.

### Turbulence
By default, there is no turbulence model, and the `effective_viscosity` is a constant viscosity in the whole domain. Optionally, the realizable k-epsilon RANS model can be used, implemented in the same way as `realizableKE` in OpenFOAM. The main purpose is to make comparisons with conventional RANS solvers easier, as the eddy viscosity has a large effect on the wakes. When the model is used, `effective_viscosity` should be the molecular viscosity.

### Boundary conditions
The outer boundaries are defined relative to the up direction of the [wind environment](../utils/wind_environment.md), which must be aligned with one of the grid axes.

| Boundary | Velocity | Pressure |
|---|---|---|
| Vertical sides | Apparent wind where it flows into the domain, zero gradient where it flows out | Zero gradient |
| Top | Zero gradient | Fixed at zero |
| Bottom | Slip | Zero gradient |

The apparent wind at the sides varies with height, as given by the `wind_condition`, `wind_environment` and `linear_velocity`. All boundaries except the top can be changed to slip walls with the `slip_wall_boundary_override` setting.

## Installation
`pystormflow` is installed with pip. It is compiled during the installation, so the [Rust toolchain](https://rust-lang.org/) must be installed first:

```bash
pip install pystormflow
```

The GPU version uses [wgpu](https://wgpu.rs/), which requires up-to-date graphics drivers with support for Vulkan, Metal or DirectX 12.

## Running a simulation
A simulation is created from a JSON string, which is most easily made with the `SimulationBuilder` in `stormbird_setup.stormflow`, as described in the [scripted setup chapter](./scripted_setup.md#stormflow). The simulation is then run until a given end time, with a time step computed from a Courant number:

```python
from pystormflow import Simulation

simulation = Simulation(simulation_builder.to_json_string())

simulation.do_steps_until_end_time(end_time=100.0, courant_number=0.75)

simulation.export_fields_as_vtk("fields.vtk", binary=True)
result = simulation.get_stormbird_simulation_result()
```

The time steps can also be executed one at a time, which is useful for monitoring the forces during the simulation:

```python
time = 0.0
while time < end_time:
    time_step = simulation.time_step_from_courant_number(0.75)
    simulation.do_step(time, time_step)
    time += time_step
```

The result from `get_stormbird_simulation_result` is a `SimulationResult` from the latest time step, with the same content as for a lifting line simulation, such as the sectional forces and the integrated forces and moments on each wing. The flow fields can be exported as a VTK file, or returned as bytes with `fields_as_vtk`, for instance to be read directly with [PyVista](https://pyvista.org/).

### Selected settings
All solver settings have default values. Some settings that are useful to know about:

- `velocity_solver.compute_platform` and `pressure_solver.settings.compute_platform`: `CPU` (default) or `GPU`.
- `velocity_solver.no_slip_wall_treatment`: `WallModel` (default) or `DataImmersion`.
- `velocity_solver.sharp_edges` and `velocity_solver.slip_sharp_edges`: the edge treatment for the no-slip and slip geometries. Turn off with `enabled=False`.
- `velocity_solver.max_velocity_factor`: an optional limit on the velocity, relative to the largest inlet velocity. This is a safeguard against a few cells with very large velocities, such as around sharp edges, which would otherwise limit the time step. It is not physically correct, so the number of limited values, given by `simulation.nr_limited_velocity_values()`, should stay small. A value of 2.5 to 3 is a reasonable starting point.
- `turbulence`: a `TurbulenceSolverBuilder` to turn on the turbulence model, with settings for the turbulence at the inlet, such as `IntensityAndViscosityRatio` (default) or `AtmosphericBoundaryLayer`.

```python
from stormbird_setup.stormflow import (
    VelocitySolverBuilder, PressureSolverBuilder, MultigridSettingsBuilder, ComputePlatform,
    TurbulenceSolverBuilder,
)

simulation_builder.velocity_solver = VelocitySolverBuilder(compute_platform=ComputePlatform.GPU)
simulation_builder.pressure_solver = PressureSolverBuilder(
    settings=MultigridSettingsBuilder(compute_platform=ComputePlatform.GPU)
)
simulation_builder.turbulence = TurbulenceSolverBuilder()
simulation_builder.effective_viscosity = 1.5e-5
```

A more complete description of the settings is given in the [Stormflow README](https://github.com/NTNU-IMT/stormbird/tree/main/stormflow).

## Command line tool
Stormflow also comes with a command line tool, mostly intended for testing, for instance to run one of the [examples](https://github.com/NTNU-IMT/stormbird/tree/main/stormflow/examples). It is run with a JSON setup file:

```bash
stormflow --file-path setup.json --end-time 100.0
```

Other options are the Courant number (`-c`), the number of CPU cores (`-n`), the wind direction (`-w`) and the internal state of each wing (`-s`), e.g., the rotational speed of a rotor sail. Run `stormflow --help` for all options.

## Limitations
- The cells that the forces are projected to are computed once, from the initial position of the wings. Wings that move during the simulation are therefore not supported yet. Wings that only rotate around their span line are fine, as long as the projection function is symmetric around it (`chord_factor` equal to `thickness_factor`).
- The grid has a uniform resolution, so large domains, or fine resolution close to the geometries, quickly require many cells.
