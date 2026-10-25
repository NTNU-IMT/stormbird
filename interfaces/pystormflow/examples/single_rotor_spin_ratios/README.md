# Single rotor sail at different spin ratios

> **Work in progress.** The setup of actuator line simulations of rotor sails is still being developed. The settings in this example are the current best guess, and are likely to change.

Actuator line simulations of a single rotor sail with Stormflow, for spin ratios from 0 to 4 in steps of 0.5. The results are compared against the data from Deybach et al. (2024), which is also used in the lifting line example in [`pystormbird/examples/3 - rotor_sails`](../../../pystormbird/examples/3%20-%20rotor_sails). The rotor has the same dimensions as in that example: diameter 5 m, height 30 m, standing 1.875 m above the ground.

The same cases can be run with OpenFOAM, using the example in [`cfd_interfaces/openfoam/examples/single_rotor_spin_ratios`](../../../cfd_interfaces/openfoam/examples/single_rotor_spin_ratios).

## Scripts

- `rotor_setup.py`: Shared setup, used by both this example and the OpenFOAM example. It contains the actuator line model, the geometries, the wind condition, the post-processing of the forces, and the plotting. The actuator line model is only defined here, so that both solvers use exactly the same settings.
- `run_simulations.py`: Runs the cases with Stormflow, stores the force coefficients in `results_stormflow.json` and plots them in `results_stormflow.png`. By default, all spin ratios and variants are run. Use `--spin-ratios` and `--variants` to run a subset. Results from earlier runs are kept in the results file, so individual cases can be rerun. Use `--plot-only` to only plot the stored results, and `--cpu` to run on the CPU instead of the GPU.
- `mesh_convergence_test.py`: Mesh convergence test. It runs a single spin ratio (default 3.0) for all variants, with increasing resolution at the rotor (default 4, 8, 16 and 32 cells per chord length), starting with the lowest. The force coefficients are stored in `mesh_convergence_stormflow.json`, and plotted against the resolution in `mesh_convergence_stormflow.png` every time a case is finished. Use `--cells-per-chord-length` and `--variants` to run a subset, `--plot-only` to only plot the stored results, and `--cpu` to run on the CPU. The grid is uniform, so the number of cells increases with the cube of the resolution, and the highest resolutions require a lot of memory.

## Variants

The actuator line model is run both with and without the lifting line correction. Everything else is the same, including the section model.

## Settings

- The geometry follows the experiments: the rotor stands on a box shaped foundation with the same width as the diameter, and has an end disk at the top with a diameter of 1.5 times the rotor diameter. Both are modelled as slip surfaces.
- The section model is a `RotatingCylinder` with a linear lift, i.e., the same lift slope at all spin ratios, tuned as described below. The drag uses the default values in Stormbird, as the point is to see how well the drag is predicted with "raw" settings.
- The lifting line correction uses `max_correction_ratio = 1.0`, which limits the velocity correction to the magnitude of the sampled velocity. Without the limit, the correction becomes larger than the freestream velocity at the ends of the rotor, which reduces the lift considerably, and makes the simulations with the end disk unstable.
- The rotor speed is set by a controller with a constant set point, given directly in revolutions per second, so that the spin ratio is defined relative to the freestream velocity, as in the data. The rotor speed is ramped up from zero over the first 5 s.
- The density in the line force model is 1.0, as both solvers are incompressible, with the pressure divided by the density.
- The forces are averaged over the last 10 s of the 40 s simulation. Tests with OpenFOAM showed that the forces converge after about 10 s with the lifting line correction, and after about 20 s without it.
- The grid has 16 cells per chord length (i.e., per diameter) at the rotor, set by `CELLS_PER_CHORD_LENGTH` in `rotor_setup.py`, which is also used for the refinement around the rotor and the geometries in the OpenFOAM example.

## Tuning of the section model

The lift slope was tuned with OpenFOAM, with the lifting line correction, at 16 cells per diameter and with the geometry described above. The target is a linear fit through the origin of the experimental lift for spin ratios up to 3, which gives a lift coefficient of about 2.40 times the spin ratio. A linear fit is used rather than the raw data, as the data contains some scatter. The lift slope in the section model was then increased until the lift at the spin ratios 1 and 2 matched the target:

| Lift slope | cl at SR 1 / target | cl at SR 2 / target | cl at SR 3 / target |
|---|---|---|---|
| 2.40 | 0.96 | 0.94 | 0.88 |
| 2.55 | 1.02 | 1.00 | 0.92 |

The drag was then compared against the experimental drag at the same lift, rather than at the same spin ratio, to check whether the lift-induced drag is predicted correctly independently of the lift. With a lift slope of 2.55, the drag is 0.96, 0.79 and 0.85 times the experimental drag at the same lift, for the spin ratios 1, 2 and 3.

## Main issues

The main issue with actuator line simulations of rotor sails is that the forces fit the experiments at low spin ratios, but fall below them at high spin ratios:

- **Lift:** with a lift slope that gives the right lift at the spin ratios 1 and 2, the lift at a spin ratio of 3 is about 8% too low, even though the section model is linear. The lift-induced velocities seem to grow faster with the lift than in reality.
- **Lift-induced drag:** the drag at a given lift is about right at low lift, but 15-20% too low at the spin ratios 2 and 3. It is not yet clear how much of this is due to the model, and how much is due to differences between the simulations and the experiments. For instance, the foundation and the end disk are frictionless slip surfaces in the simulations, and only the force on the actuator line is included.
- **Stability:** the lifting line correction without a limit gives very large velocity corrections at the ends of the rotor, as the reference model with the small viscous core is close to singular there. This reduces the lift, and can make the simulations unstable, in particular with an end disk. The `max_correction_ratio` limits this.

Large spin ratios give very large lift coefficients, and the actuator line simulations may become unstable at the largest spin ratios.

## Requirements

The `max_correction_ratio` setting is not part of Stormbird 0.11.0. Until it is released, pystormflow must be built against the local version of the Stormbird library in this repository.
