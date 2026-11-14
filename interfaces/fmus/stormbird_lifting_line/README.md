# Functional Mockup Interface (FMI) to run lifting line simulations

This crate generates a Functional Mockup Unit (FMU) that can execute lifting line simulations in a
dynamic situation, possibly together with other models that support the
[FMI-standard](https://fmi-standard.org/). The FMU can be built for either version 2 or version 3 of
the standard, with version 2 as the default.

The FMU is built around the `CompleteSailModel` in Stormbird, which combines a lifting line
simulation with a wind environment and an optional controller. See the
[Stormbird book](https://jarlekramer.github.io/stormbird_book_pages/) for more information about the
model and about how to use the FMU.

## Build instructions
The interface is automatically generated using the `Fmu` derive macro from
[fmu_from_struct](https://github.com/jarlekramer/fmu_from_struct). To build and package the FMU:

- Install the cargo subcommand that builds and packages FMUs, by running
  `cargo install cargo-fmu-from-struct`
- Run `cargo fmu-from-struct --release` in this folder. This compiles the library and packages it
  as `target/fmu/release/StormbirdLiftingLine.fmu`. All arguments except `--resources` and
  `-o/--output` are passed on to `cargo build`, so features can be activated as usual, e.g.,
  `cargo fmu-from-struct --release --features parallel`
- The version of the FMI standard is selected with the cargo features `fmi2` (default) and `fmi3`.
  Exactly one of them must be active. To build an FMU for version 3, turn off the default feature:
  `cargo fmu-from-struct --release --no-default-features --features fmi3`. Both versions are
  written to the same path by default, so use `-o/--output` to keep both, e.g.,
  `-o target/fmu/release/StormbirdLiftingLine_fmi3.fmu`. Version 2 is the most tested version.
- Optional: to optimize specifically for the local CPU, first set the environment variable
  `RUSTFLAGS="-C target-cpu=native"`. The FMU will then not necessarily run on other computers.
- Optional: use `--resources <PATH>` to include a folder as the `resources` folder in the FMU, for
  instance with a `parameters.json` file (see below). By default, the `resources` folder in this
  crate is used, if it exists.
- Copy/move the resulting `StormbirdLiftingLine.fmu` to wherever you like, and load it using your
  preferred FMI simulator to execute. This can for instance be
  [FMPy](https://github.com/CATIA-Systems/FMPy) or the simulator from
  [Open Simulation Platform](https://opensimulationplatform.com/)

A plain `cargo build` compiles the library and writes the model description to
`target/fmu/stormbird_lifting_line/modelDescription.xml`, where it can be inspected, but does not
package the FMU.

## Input
The FMU has two parameters:

- `parameters_path`: path to a JSON file with the parameters for the model. If empty, the file
  `parameters.json` in the resources folder of the FMU is used.
- `time_model_scale`: if larger than zero, the time and time step are scaled from model scale to
  full scale with the square root of this value. This is a deliberate workaround for hybrid
  experiments, where all FMUs in the system share the same real time, so a scaled time can not be
  given to a single FMU. **Only the time is scaled.** All other inputs, such as the translation,
  the motion velocities and the wind velocity, are used as they are, and the forces and moments are
  given as they are. The normal way to use the FMU in hybrid experiments is to do all other
  scaling between model scale and full scale outside the FMU, e.g., in dedicated logic between the
  experiment and the simulation. Note that the velocities computed from the translation and
  rotation inputs with finite differences use the scaled time step, so the translation must be
  given in full scale.

## Motion
The translation and rotation inputs set the position of the sails. The rotation inputs are the
angles around the x, y and z axes, applied in the order given by the `rotation_type` in the line
force model (default: around x, then y, then z, i.e., the same as yaw, pitch and roll). The
velocities of the motion are computed from the change in the translation and rotation with finite
differences. The angular velocity is computed from the change in the rotation matrix, so it is the
angular velocity vector, which is correct also when several angles are different from zero, e.g., a
roll motion with a heading.

In the first time step, there is no previous position, so the velocities are zero. The motion is
tracked in every time step, also while waiting for `number_of_iterations_before_building_model`.

If `use_motion_velocity` is true, the velocity inputs are used instead of the computed velocities.
The `motion_velocity_angular_*` inputs are the angular velocity *vector*, not the time derivative of
the rotation angles. With `motion_velocity_in_body_fixed_frame`, both the linear and the angular
velocity inputs are given in the body fixed coordinate system, e.g., (u, v, w) and (p, q, r) from a
ship simulator. Otherwise, they are given in the global coordinate system.

The sails are simulated as long as the velocity they feel, i.e., the wind minus the motion of the
sails, is not zero. A moving ship in calm wind therefore gives forces.

The parameters file contains the complete sail model, under `sail_model`, and settings that are
specific to the FMU. Only `sail_model` is required:

```json
{
    "sail_model": {
        "lifting_line_simulation": { ... },
        "wind_environment": { ... },
        "controller": { ... },
        "settings": { ... }
    },
    "angles_in_degrees": false,
    "use_motion_velocity": false,
    "motion_velocity_in_body_fixed_frame": false,
    "use_motion_velocity_linear_as_freestream": false,
    "input_moving_average_window_size": 0,
    "number_of_iterations_before_building_model": 0
}
```

The `sail_model` is the JSON representation of a `CompleteSailModelBuilder`, which can be generated
with the `CompleteSailModelBuilder` class in the
[stormbird_setup](https://pypi.org/project/stormbird_setup/) Python package. Only the
`lifting_line_simulation` is required. The `wind_environment` is the default wind environment if
not given. The `controller` is optional. Without a controller, the local wing angles and internal
states are taken from the inputs of the FMU at every time step. With a controller, the inputs are
only used as initial values.

The FMU can model up to 10 sails, which is the number of inputs and outputs for each sail.

## Error codes
The FMU is designed to not crash, or return an error status to the importer, due to errors in the
input or in the simulation. This is because errors in the input are common when an FMU is set up,
and not all software that runs FMUs handles failing FMUs well. Instead, errors are reported with the
`error_code` output, which is zero when everything is OK. The error message is printed to the
standard error output.

After an error, the model is not simulated anymore, and all other outputs are zero, until the FMU
is restarted. An error is an indication that something is wrong with the setup, which should be
looked into before running again.

| Code | Meaning                                                                       |
|------|-------------------------------------------------------------------------------|
| 0    | No error                                                                      |
| 1    | IO error, e.g., the parameters file could not be found                        |
| 2    | JSON error, e.g., a syntax error or an unknown field in the parameters file   |
| 3    | Invalid input, e.g., an inconsistent setup of the sail model                  |
| 4    | Other error from Stormbird                                                    |
| 10   | Unexpected error (panic) while setting up the model                           |
| 11   | Unexpected error (panic) while simulating a time step                         |
| 12   | The simulation gave forces or moments that are not finite                     |
| 13   | The sail model has more than 10 sails                                         |
| 14   | A time step was simulated before the model was set up                         |
| 15   | The input gave a freestream velocity that is not finite, e.g., a NaN input    |

The codes from 1 to 4 are the same as the codes from `stormbird::error::Error::code`. Codes 10 and
11 indicate a bug in Stormbird or in the FMU, and should be reported.
