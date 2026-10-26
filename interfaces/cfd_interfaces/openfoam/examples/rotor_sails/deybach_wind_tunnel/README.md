# Actuator line simulations of rotor sails with OpenFOAM

> **Work in progress.** The setup of actuator line simulations of rotor sails is still being developed. See the README of the Stormflow version of these examples for the current status.

This is the OpenFOAM version of the Stormflow examples in [`pystormflow/examples/rotor_sails/deybach_wind_tunnel`](../../../../../pystormflow/examples/rotor_sails/deybach_wind_tunnel). The Stormflow examples are the reference, and the README there describes the goals of the examples, the challenges with rotor sails, and the variants of the actuator line model.

The examples here are kept as close as possible to the Stormflow version. Everything except the mesh is imported from the Stormflow folder:

- `rotor_setup.py`: the actuator line model, the domain (the wind tunnel from the experiments), the flow settings and the post-processing of the forces.
- `plotting.py`: the storage of the results as time series, and all the plots. The results are stored in this folder, in the same format as for Stormflow.

## Differences from Stormflow

The only intended difference is the mesh. Stormflow uses a uniform grid, while the OpenFOAM mesh has a uniform background mesh (blockMesh) that is refined in a cylinder around each rotor (snappyHexMesh):

- The resolution in the refinement zone is `CELLS_PER_CHORD_LENGTH` in `rotor_setup.py`, which is the same as the uniform grid in Stormflow.
- The background mesh is coarser by the factor `BACKGROUND_COARSENING_FACTOR` (2) in `run_single_rotor.py`. The number of cells per chord length must therefore be divisible by 2.
- The radius of the refinement zone is about one diameter, set by `REFINEMENT_PADDING` in `run_single_rotor.py`.

The flow settings match Stormflow. The OpenFOAM simulation builder does not expose all of them, so they are changed directly in the case files after the case is written:

- No turbulence model (`simulationType laminar`), and the same constant viscosity as in Stormflow (`EFFECTIVE_VISCOSITY` in `rotor_setup.py`).
- The time step is set by the same maximum Courant number (`COURANT_NUMBER` in `rotor_setup.py`), with `MAX_TIME_STEP` in `run_single_rotor.py` as the initial and maximum time step.
- The wind tunnel boundary conditions, with an inlet, an outlet, and slip walls on the sides, the top and the ground.

## Running

OpenFOAM and the [OpenFOAM interface](../../../README.md) must be installed, and OpenFOAM must be loaded in the terminal. The cases are written to `$FOAM_RUN/stormbird_rotor_sails`, in folders with the same names as the time series they produce. An existing case is replaced when it is run again. All the scripts take `--nr-processes` to set the number of processes. The default is the number of physical cores.

The scripts have the same names and options as in the Stormflow version:

- `run_single_rotor.py`: Runs a single rotor for different spin ratios.
- `mesh_convergence_test.py`: Mesh convergence test for a single spin ratio. The background mesh is refined together with the resolution at the rotor, so the ratio between them is the same for all resolutions.
- `run_two_rotors.py`: Runs two rotors, with the interfering rotor at different angular positions around the other. Use `--angular-positions` to run a few positions instead of all the positions from the experiments.
- `plot_results.py`: Plots the results with `plotting.py` from the Stormflow folder, with OpenFOAM as the default solver. It has the same commands as `plotting.py`. It is not called `plotting.py`, as it would then hide the module it imports.

All the simulation scripts only run the recommended variant by default. Use `--variant` to choose one or more other variants, or `--variant all` to run all of them.

```bash
python run_single_rotor.py
python plot_results.py means

python mesh_convergence_test.py
python plot_results.py mesh-convergence

python run_two_rotors.py --angular-positions 0 30 90
python plot_results.py interference
```

The examples use settings that require Stormbird 0.12.0 or newer, such as the `application` setting in the tip loss correction, which is used by the default variant. The OpenFOAM interface is built against the local version of Stormbird in this repository, so it must be built, with `build.sh`, from the same version of the repository as the examples.
