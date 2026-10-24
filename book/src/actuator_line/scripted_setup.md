# Scripted setup

The [stormbird_setup](https://github.com/NTNU-IMT/stormbird/tree/main/interfaces/stormbird_setup) Python library can create the input for actuator line simulations with both [Stormflow](./stormflow.md) and the [OpenFOAM interface](./openfoam_interface.md). In both cases, the setup is made through a `SimulationBuilder` class, found in the `stormbird_setup.stormflow` and `stormbird_setup.openfoam` modules, respectively. The two builders are kept as similar as possible.

## Common settings

- `actuator_line`: an `ActuatorLineBuilder`, as described in the [simulation overview](./simulation_overview.md).
- `wind_condition` and `linear_velocity`: the true wind and the velocity due to the motion of the ship, which are added together to give the apparent wind in the domain. The optional `wind_environment` field controls how the wind condition is interpreted. See the [wind environment chapter](../utils/wind_environment.md).
- `geometries` and `slip_geometries`: other structures in the domain, with no-slip and slip walls respectively. Typically created with `GeometryBuilder.new_triangle_mesh(file_path)`.
- `grid`: the domain size and resolution. The resolution is specified as a number of cells per *representative length*, which by default is the average chord length of the wings. It can be set manually with `grid.representative_length`, which is required if there is no actuator line model.

## Stormflow

Stormflow uses a uniform grid:

```python
from stormbird_setup.spatial_vector import SpatialVector
from stormbird_setup.stormflow import SimulationBuilder, GridBuilder, GeometryBuilder, WindCondition

simulation_builder = SimulationBuilder(
    grid=GridBuilder(
        start_point=SpatialVector(x=-128.0, y=-64.0, z=0.0),
        end_point=SpatialVector(x=128.0, y=64.0, z=64.0),
        cells_per_representative_length=(11, 11, 11),
    ),
    wind_condition=WindCondition(direction_coming_from=1.57, velocity_variation=8.0),
    linear_velocity=SpatialVector(x=6.0, y=0.0, z=0.0),
    actuator_line=actuator_line_builder,
    geometries=[GeometryBuilder.new_triangle_mesh("superstructure.obj")],
)
```

The builder also contains the solver settings (`velocity_solver`, `pressure_solver`, `solver_settings`, `turbulence` and `effective_viscosity`), which all have default values. The JSON string from the builder is the direct input to Stormflow:

```python
from pystormflow import Simulation

simulation = Simulation(simulation_builder.to_json_string())
```

## OpenFOAM

Settings such as schemes, solvers, turbulence model and viscosity are not part of the builder. They are taken from a *base folder*, which is a complete OpenFOAM case. A default base folder is included in the package, and a custom one can be used by setting `base_folder` to its path.

The mesh consists of a background mesh, made with `blockMesh`, and optional refinement around the actuator lines and the geometries, made with `snappyHexMesh`:

```python
from stormbird_setup.spatial_vector import SpatialVector
from stormbird_setup.openfoam import (
    SimulationBuilder, GridBuilder, RefinementBuilder, GeometryBuilder, WindCondition
)

simulation_builder = SimulationBuilder(
    grid=GridBuilder(
        start_point=SpatialVector(x=-200.0, y=-200.0, z=0.0),
        end_point=SpatialVector(x=200.0, y=200.0, z=200.0),
        background_mesh_cells_per_representative_length=1,
        actuator_line_refinement=RefinementBuilder(cells_per_representative_length=8, padding=1.0),
        geometry_cells_per_representative_length=4,
    ),
    wind_condition=WindCondition(direction_coming_from=1.57, velocity_variation=8.0),
    linear_velocity=SpatialVector(x=6.0, y=0.0, z=0.0),
    actuator_line=actuator_line_builder,
    geometries=[GeometryBuilder.new_triangle_mesh("ship.obj")],
)
```

Grid settings:

- `background_mesh_cells_per_representative_length`: resolution of the background mesh.
- `actuator_line_refinement`: refinement zone around each wing, shaped as a cylinder from the first to the last section point. The diameter is `padding` × the mean chord length of the wing × the largest factor in the Gaussian projection function. `None` means no refinement.
- `geometry_cells_per_representative_length`: resolution at the surface of the geometries. How far the refinement extends away from the surfaces is controlled by the `snappyHexMesh` settings in the base folder. `None` means no refinement.

Since `snappyHexMesh` refines by splitting cells in two, the actual resolution in a refined region is the background resolution multiplied by a power of two, at least as fine as the requested value.

The number of cells in the background mesh is rounded up along each axis, so that the cells fit the domain exactly and are at least as fine as requested.

All geometries are snapped to, without boundary layers. Supported geometries are triangle meshes, spheres and cuboids. Each geometry becomes a patch in the mesh, named `geometry_<index>` for `geometries` (type `wall`) and `slip_geometry_<index>` for `slip_geometries` (type `patch`), where the index is the position in the list. Triangle mesh files are copied to `constant/triSurface`, and should contain a single group or solid. Feature edges are captured with implicit feature snapping.

### Time control

The `time_control` field selects between a steady simulation with `simpleFoam` and a time dependent simulation with `pimpleFoam`:

```python
from stormbird_setup.openfoam import SteadyTimeControl, DynamicTimeControl

steady = SteadyTimeControl(max_iterations=1000)
dynamic = DynamicTimeControl(time_step=0.01, end_time=60.0, max_courant_number=1.0)
```

- `SteadyTimeControl` (default): `max_iterations` (default 1000). The simulation stops earlier if the residuals converge.
- `DynamicTimeControl`: `time_step` and `end_time`, and `max_courant_number` (default 1). The time step is reduced if necessary to keep the Courant number below the maximum, but never exceeds `time_step`.

The fields are written 10 times during the simulation, and only the latest write is kept (`purgeWrite 1`). The base folder contains a steady and a dynamic version of `system/controlDict` and `system/fvSchemes`, marked with the suffixes `.steady` and `.dynamic`, and the matching version is used in the case.

### Boundary conditions

| Boundary | Velocity | Pressure | Turbulence |
|---|---|---|---|
| Vertical sides | Apparent wind where it flows into the domain, zero gradient where it flows out | Zero gradient | Inlet value where the flow enters, zero gradient where it leaves |
| Top | Zero gradient | Fixed at zero | Zero gradient |
| Bottom | Slip | Slip | Slip |
| `geometries` | No slip | Zero gradient | Wall functions |
| `slip_geometries` | Slip | Slip | Slip |

The apparent wind at the vertical sides varies with height, as given by the `wind_condition`, `wind_environment` and `linear_velocity`. It is also used as the initial velocity in the domain, evaluated at 10 m height. Since the base folder assumes that z is up, `wind_environment.up_direction` must be `(0, 0, 1)`. Gusts and inflow corrections are not supported.

The turbulence is set by `inlet_turbulence`, which is an `IntensityAndViscosityRatio` with the fields `turbulence_intensity` (default 0.01) and `eddy_viscosity_ratio` (default 10). The values are the same at all heights, and are computed as `k = 1.5 (I U)^2` and `omega = k / (r nu)`, where `U` is the true wind velocity at 10 m height, or the magnitude of `linear_velocity` if there is no true wind, and `nu` is the viscosity in the base folder.

### Writing the case

```python
simulation_builder.write_foam_case("path/to/case")
```

This copies the base folder to the given path, which must not already exist, and applies the settings from the builder. The actuator line model is written to `system/stormbird_actuator_line.json`, and is loaded by the [OpenFOAM interface](./openfoam_interface.md), which must be installed.

### Running the case

```python
simulation_builder.run_foam_case("path/to/case")
```

This meshes the case with `blockMesh` and `snappyHexMesh`, and runs the solver. It waits until the simulation is finished, and raises an error if it fails. OpenFOAM must be loaded in the terminal that runs the script. The solver runs in parallel on the number of physical cores in the computer, excluding hyperthreading. This can be changed with the optional `nr_processes` argument, and `system/decomposeParDict` is updated to match. The meshing always runs on a single process. The output from each step is written to log files in the case folder, such as `log.snappyHexMesh` and `log.simpleFoam`.

The same steps can be executed directly with the run script in the case folder:

```bash
bash run.sh      # Number of physical cores
bash run.sh 8    # 8 processes
```

### Reading the results

```python
results = simulation_builder.read_results("path/to/case")
```

This returns an `ActuatorLineResults` object with the [results from the actuator line model](./openfoam_interface.md#results):

- `forces`: the columns in `stormbird_forces.csv` as lists, with the column names as keys, e.g. `results.forces["force_0.x"]`.
- `full_results`: the full simulation results as dictionaries, with the iteration number as key.

The results can be stored with `results.to_json_file(file_path)`, and read again with `ActuatorLineResults.from_json_file(file_path)`.

<!-- TODO: Content of the default base folder -->
