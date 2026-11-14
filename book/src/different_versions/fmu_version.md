# Functional Mockup Unit

The lifting line functionality in Stormbird is available as a *Functional Mockup Unit*, which means that the functionality can be executed through the [Functional Mockup Interface standard](https://fmi-standard.org/). The actual interface is generated using the [fmu_from_struct](https://github.com/jarlekramer/fmu_from_struct) library, which is made by the same developer as Stormbird.

## How to execute simulations using the FMU-version
The FMU can be built for either version 2 or version 3 of the FMI-standard, which is chosen when the FMU is built (see [below](#building-the-fmu)). Version 2 is the default. This choice is made because the developers of Stormbird primarily use the [Open Simulation Platform](https://opensimulationplatform.com/) for executing simulations, which currently only supports version 2. Version 2 is therefore also the most tested version. The variables and the behavior of the FMU are the same for both versions.

A simulation can be executed with any simulation platform that supports the FMI-standard. One example is the [command line interface](https://open-simulation-platform.github.io/cosim) from the Open Simulation Platform, or some Python interface to FMUs such as [FMPy](https://github.com/CATIA-Systems/FMPy) or [PyFMI](https://jmodelica.org/pyfmi/).

To actually set up a simulation, it is necessary to pass in several parameter- and input variables to the FMU unit. The most important ones are described below. For a full overview of the available variables, see the [FMU source code](https://github.com/NTNU-IMT/stormbird/tree/main/interfaces/fmus/stormbird_lifting_line). To see how it can be used in practice, see the [FMU example folder](https://github.com/NTNU-IMT/stormbird/tree/main/interfaces/fmus/examples).

## Building the FMU
The FMU is not distributed as a ready-made file, and must be built from the source code. This requires a [Rust installation](https://www.rust-lang.org/tools/install). The FMU is built and packaged with the `cargo fmu-from-struct` subcommand from the fmu_from_struct library:

```
cargo install cargo-fmu-from-struct
cd interfaces/fmus/stormbird_lifting_line
cargo fmu-from-struct --release
```

This gives the file `target/fmu/release/StormbirdLiftingLine.fmu`, which supports version 2 of the FMI-standard. To build an FMU for version 3 instead, select the `fmi3` feature, and turn off the default `fmi2` feature:

```
cargo fmu-from-struct --release --no-default-features --features fmi3
```

See the [README of the FMU](https://github.com/NTNU-IMT/stormbird/tree/main/interfaces/fmus/stormbird_lifting_line) for more options.

## Setting up the model
The sails are modeled with a [complete sail model](../lifting_line/complete_sail_model.md), which combines a dynamic lifting line simulation with a wind environment and an optional controller. The FMU has two parameters:

- `parameters_path`: the path to a JSON file with the parameters for the model. If it is empty, the file `parameters.json` in the resources folder of the FMU is used.
- `time_model_scale`: if larger than zero, the time and time step are scaled from model scale to full scale with the square root of this value. This is a deliberate workaround for hybrid experiments. All FMUs in a system share the same real time, so it is not possible to give a scaled time to a single FMU. **Only the time is scaled**. All other inputs, such as the translation, the motion velocities and the wind velocity, are used as they are, and the forces and moments are given as they are. The normal way to use the FMU in hybrid experiments is to do all other scaling between model scale and full scale outside the FMU, e.g., in dedicated logic between the experiment and the simulation. Note that the velocities that are computed from the translation and rotation with finite differences use the scaled time step, so the translation must be given in full scale.

The parameters file contains the complete sail model, under the key `sail_model`, together with a few settings that are specific to the FMU:

```json
{
    "sail_model": {
        "lifting_line_simulation": { ... },
        "wind_environment": { ... },
        "controller": { ... }
    },
    "angles_in_degrees": false,
    "use_motion_velocity": false,
    "motion_velocity_in_body_fixed_frame": false,
    "use_motion_velocity_linear_as_freestream": false,
    "input_moving_average_window_size": 0,
    "number_of_iterations_before_building_model": 0,
    "steady_state": false,
    "steady_state_optimize_loading": false
}
```

Only the `sail_model`, and the `lifting_line_simulation` within it, are required. The `sail_model` can be generated with the `CompleteSailModelBuilder` in [stormbird_setup](./io_overview.md), for instance with `to_dict()`. The lifting line simulation should normally use dynamic simulation settings, as the FMU is used for time-domain simulations.

The settings that are specific to the FMU are:

- `angles_in_degrees`: whether the angles in the inputs and outputs are given in degrees, rather than radians.
- `use_motion_velocity`: whether the motion velocity inputs should override the velocities that are computed from the translation and rotation inputs with finite differences.
- `motion_velocity_in_body_fixed_frame`: whether the motion velocity inputs, both linear and angular, are given in the body-fixed coordinate system, e.g., (u, v, w) and (p, q, r) from a ship simulator, rather than the global one.
- `use_motion_velocity_linear_as_freestream`: whether the linear motion velocity should be included in the apparent wind, rather than as motion of the sails. This should only be used if the translation input is always zero.
- `input_moving_average_window_size`: if larger than zero, the motion inputs are filtered with a moving average over this number of time steps.
- `number_of_iterations_before_building_model`: if larger than zero, the sails are not simulated before this number of time steps are completed. This is useful if the inputs are not properly set in the first time steps.
- `steady_state`: whether each time step should be an independent steady state simulation, rather than a step in a time-domain simulation. See below.
- `steady_state_optimize_loading`: whether the steady state simulations should search for the controller loading that gives the highest effective power. Only allowed together with `steady_state`.

### Time-domain and steady state mode
By default, the FMU runs a time-domain simulation, where each call to `do_step` is a time step in the lifting line simulation, with the time and time step from the importer. This works with both quasi-steady and dynamic settings for the lifting line simulation. That is, the sails can be modeled with a quasi-steady wake in a time-domain simulation, e.g., to save computational time, while the time, gusts and the controller still behave as in the time domain. The controller, if any, is updated once per time step, using all its settings, such as input filters and rate limits, so it may need several time steps to reach the set points.

With `steady_state` set to true, each call to `do_step` is instead an independent steady state simulation, using the [steady state methods](../lifting_line/complete_sail_model.md#steady-state-simulations) of the complete sail model. This is intended for cases where the ship is modeled in steady conditions, e.g., when a ship simulator searches for the steady speed of the ship. In this mode:

- The lifting line simulation must use quasi-steady settings. Dynamic settings give error code 3.
- The sails are placed at the input translation and rotation, without any motion velocity.
- The ship velocity is the linear motion velocity input, in the global or body fixed coordinate system, depending on `motion_velocity_in_body_fixed_frame`.
- With a controller, the model iterates between simulations and controller updates until the controller output has converged, with the `controller_loading` input as the loading. Settings that are only meaningful in the time domain, such as input filters and rate limits, are not used. Without a controller, the local wing angles and internal states from the inputs are used.
- With `steady_state_optimize_loading`, the loading that gives the highest effective power is searched for, up to the `controller_loading` input.

The result of each time step is then independent of the time, the time step and the earlier time steps.

## Inputs and outputs
The inputs to the FMU are, in short:

- The wind velocity, `wind_velocity`, and the direction the wind is coming from, `wind_direction_coming_from`, which are used to create the wind condition in each time step. The variation of the wind velocity with height follows a power model, where the wind velocity is the velocity at the default reference height of 10 meters.
- The translation and rotation of the sails, and, optionally, the velocity of the motion. The rotation inputs are the angles around the x, y and z axes, applied in the order given by the `rotation_type` of the line force model (see [moving line models](../line_model/move_line_models.md)). The velocities are computed from the change in translation and rotation with finite differences, where the angular velocity is computed from the change in the rotation matrix. It is therefore the angular velocity vector, which is correct also when several angles are different from zero, e.g., for a roll motion with a heading. In the first time step, there is no previous position, so the velocities are zero. The motion is tracked in every time step, also while waiting for `number_of_iterations_before_building_model`. The angular motion velocity inputs, if used, must also be the angular velocity vector, and not the time derivative of the rotation angles.
- The local wing angles and the internal states of each sail. Without a controller, these are used in every time step. With a controller, they are only used as initial values, and the sails are controlled by the controller after the first time step. The `controller_loading` input sets the loading of the controller.

The outputs are the total forces and moments, the forces and moments on each sail, the power consumed by the sails, in total, `input_power`, and for each sail, `input_power_sail_1` to `input_power_sail_10`, the angle of attack, velocity and apparent wind direction measured on each sail, and an error code (see below). The consumed power is computed with the [input power model](../line_model/input_power.md) of each wing, e.g., for the motors of rotor sails or the fans of suction sails, and is zero for sails that do not need power. The FMU supports up to 10 sails, which is the number of inputs and outputs for each sail.

The sails are simulated in each time step where the velocity they feel, i.e., the wind minus the motion of the sails, is not zero. A moving ship in calm wind therefore gives forces.

## Error handling
A typical use case for the FMU is that it is set up with small errors in the input. Some of the software that runs FMUs does not handle failing FMUs well, so the Stormbird FMU is designed to not crash, or return an error status, due to errors in the input or in the simulation. Instead, errors are reported with the `error_code` output, which is zero when everything is OK. The error message is printed to the standard error output.

After an error, the sails are not simulated anymore, and all the other outputs are zero, until the FMU is restarted. An error is an indication that something is wrong with the setup, which should be looked into before the simulation is run again.

| Code | Meaning                                                                       |
|------|-------------------------------------------------------------------------------|
| 0    | No error                                                                      |
| 1    | IO error, e.g., the parameters file could not be found                        |
| 2    | JSON error, e.g., a syntax error or an unknown field in the parameters file   |
| 3    | Invalid input, e.g., an inconsistent setup of the sail model                  |
| 4    | Other error from Stormbird                                                    |
| 10   | Unexpected error (panic) while setting up the model                           |
| 11   | Unexpected error (panic) while simulating a time step                         |
| 12   | The simulation gave forces, moments or input power that are not finite        |
| 13   | The sail model has more than 10 sails                                         |
| 14   | A time step was simulated before the model was set up                         |
| 15   | The input gave a freestream velocity that is not finite, e.g., a NaN input    |

Codes 10 and 11 indicate a bug in Stormbird or in the FMU, and should be reported.

## What can the FMU-version do?
The FMI-interface inherently comes with more limitations than a conventional API, such as the Python interface. For instance, there are limitations on what type of variables that can be passed to and from different FMUs, and there is a specific order to how functions are executed. Although there are many ways to work around these limitations, the FMU-version of Stormbird is, for simplicity's sake, designed to only cover the most typical use cases for running dynamic lifting line simulations. That is, **it is not intended to be a direct alternative to the Python interface**, but rather a specialized way to use *some* of the functionality.

To be more specific, there are essentially two primary use cases for the Stormbird FMU:

1) **Coupling of Stormbird to a time-domain ship simulator**, such as [VeSim](https://www.sintef.no/en/software/vesim/). VeSim is a ship simulator that includes maneuvering and seakeeping models of ships. It can, for instance, simulate a ship moving in waves, including the effect of rudder action and control systems. The software is built around the FMI-standard to couple different sub-models together. Stormbird can, therefore, be one of many models in a ship-system simulation in the time-domain.
2) **Running sail simulations in hybrid experiments**. Hybrid experiments are experiments where part of the physics is measured experimentally, while other parts are simulated. In the specific case of wind-powered ships, the aerodynamic forces on the sails are simulated while the hydrodynamics are tested in a towing tank. [This article](https://www.sciencedirect.com/science/article/pii/S0029801821015213?via%3Dihub) explains more of how this is done at SINTEF Ocean. The Stormbird FMU was designed to fit well with the laboratory software used at SINTEF Ocean when doing hybrid tests, called [HLCC](https://www.sintef.no/programvare/hlcc/).

There is no direct coupling to VeSim or HLCC in the Stormbird FMU, but the choice of input and output variables was made, in part, based on what makes sense for these external software packages. That is, the design of the Stormbird FMU interface is not made in isolation.
