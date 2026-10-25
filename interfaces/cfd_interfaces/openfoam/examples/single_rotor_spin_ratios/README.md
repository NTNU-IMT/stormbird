# Single rotor sail at different spin ratios

> **Work in progress.** The setup of actuator line simulations of rotor sails is still being developed. See the README of the Stormflow version of this example for the current status, how the section model was tuned, and the main issues.

Actuator line simulations of a single rotor sail with OpenFOAM, for spin ratios from 0 to 4 in steps of 0.5, both with and without the lifting line correction. The results are compared against the data from Deybach et al. (2024).

This is the OpenFOAM version of the Stormflow example in [`pystormflow/examples/single_rotor_spin_ratios`](../../../../pystormflow/examples/single_rotor_spin_ratios). See the README there for a description of the cases and the settings. The actuator line model, the foundation and the end disk are imported from `rotor_setup.py` in that folder, so that both solvers use exactly the same settings. Only the mesh is different:

- The domain is larger than in Stormflow, with a background mesh of 1 cell per diameter.
- The mesh is refined to 16 cells per diameter in a cylinder around the rotor and at the surfaces of the foundation and the end disk, which is the same resolution as the uniform grid in Stormflow. Both use `CELLS_PER_CHORD_LENGTH` in `rotor_setup.py`.

## Running

OpenFOAM and the [OpenFOAM interface](../../README.md) must be installed, and OpenFOAM must be loaded in the terminal. The cases are written to `$FOAM_RUN/stormbird_single_rotor_spin_ratios`.

The lifting line correction uses the `max_correction_ratio` setting, which is not part of Stormbird 0.11.0. Until it is released, the OpenFOAM interface must be built against the local version of the Stormbird library in this repository.

```bash
python run_simulations.py
```

The force coefficients are stored in `results_openfoam.json` and plotted in `results_openfoam.png`. Use `--spin-ratios` and `--variants` to run a subset of the cases, `--nr-processes` to set the number of processes, and `--plot-only` to only plot the stored results.

## Mesh convergence test

`mesh_convergence_test.py` runs a single spin ratio (default 3.0) for all variants, with increasing resolution at the rotor (default 4, 8, 16 and 32 cells per chord length), starting with the lowest. The background mesh is the same for all resolutions. The force coefficients are stored in `mesh_convergence_openfoam.json`, and plotted against the resolution in `mesh_convergence_openfoam.png` every time a case is finished. Use `--cells-per-chord-length` and `--variants` to run a subset, `--nr-processes` to set the number of processes, and `--plot-only` to only plot the stored results.

```bash
python mesh_convergence_test.py
```
