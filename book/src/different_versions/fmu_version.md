# Functional Mockup Unit

The lifting line functionality in Stormbird is available as a *Functional Mockup Unit*, which means that the functionality can be executed through the [Functional Mockup Interface standard](https://fmi-standard.org/). The actual interface is generated using the [fmu_from_struct](https://github.com/jarlekramer/fmu_from_struct) library, which is made by the same developer as Stormbird.

## How to execute simulations using the FMU-version
The FMU-version is currently made to support version 2 of the FMI-standard. This choice is made because the developers of Stormbird primarily use the [Open Simulation Platform](https://opensimulationplatform.com/) for executing simulations, which currently only supports version 2. An FMU that supports version 3 of the FMI-standard is relatively straightforward to make, but will probably not be prioritized before the Open Simulation Platform is updated to the latest version.

A simulation can be executed with any simulation platform that supports the FMI-standard. One example is the [command line interface](https://open-simulation-platform.github.io/cosim) from the Open Simulation Platform, or some Python interface to FMUs such as [FMPy](https://github.com/CATIA-Systems/FMPy) or [PyFMI](https://jmodelica.org/pyfmi/).

To actually set up a simulation, it is necessary to pass in several parameter- and input variables to the FMU unit. The most important ones are described below. For a full overview of the available variables, see the [FMU source code](https://github.com/NTNU-IMT/stormbird/tree/main/interfaces/fmus/stormbird_lifting_line). To see how it can be used in practice, see the [FMU example folder](https://github.com/NTNU-IMT/stormbird/tree/main/interfaces/fmus/examples).

## Building the FMU
The FMU is not distributed as a ready-made file, and must be built from the source code. This requires a [Rust installation](https://www.rust-lang.org/tools/install). The FMU is built and packaged with the `cargo fmu-from-struct` subcommand from the fmu_from_struct library:

```
cargo install cargo-fmu-from-struct
cd interfaces/fmus/stormbird_lifting_line
cargo fmu-from-struct --release
```

This gives the file `target/fmu/release/StormbirdLiftingLine.fmu`. See the [README of the FMU](https://github.com/NTNU-IMT/stormbird/tree/main/interfaces/fmus/stormbird_lifting_line) for more options.

## Setting up the model
The sails are modeled with a [complete sail model](../lifting_line/complete_sail_model.md), which combines a dynamic lifting line simulation with a wind environment and an optional controller. The FMU has two parameters:

- `parameters_path`: the path to a JSON file with the parameters for the model. If it is empty, the file `parameters.json` in the resources folder of the FMU is used.
- `time_model_scale`: if larger than zero, the time is scaled from model scale to full scale with the square root of this value. This is useful in hybrid experiments.

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
    "number_of_iterations_before_building_model": 0
}
```

Only the `sail_model`, and the `lifting_line_simulation` within it, are required. The `sail_model` can be generated with the `CompleteSailModelBuilder` in [stormbird_setup](./io_overview.md), for instance with `to_dict()`. The lifting line simulation should normally use dynamic simulation settings, as the FMU is used for time-domain simulations.

The settings that are specific to the FMU are:

- `angles_in_degrees`: whether the angles in the inputs and outputs are given in degrees, rather than radians.
- `use_motion_velocity`: whether the motion velocity inputs should override the velocities that are computed from the translation and rotation inputs with finite differences.
- `motion_velocity_in_body_fixed_frame`: whether the linear motion velocity input is given in the body-fixed coordinate system, rather than the global one.
- `use_motion_velocity_linear_as_freestream`: whether the linear motion velocity should be included in the apparent wind, rather than as motion of the sails. This should only be used if the translation input is always zero.
- `input_moving_average_window_size`: if larger than zero, the motion inputs are filtered with a moving average over this number of time steps.
- `number_of_iterations_before_building_model`: if larger than zero, the sails are not simulated before this number of time steps are completed. This is useful if the inputs are not properly set in the first time steps.

## Inputs and outputs
The inputs to the FMU are, in short:

- The wind velocity, `wind_velocity`, and the direction the wind is coming from, `wind_direction_coming_from`, which are used to create the wind condition in each time step. The variation of the wind velocity with height follows a power model, where the wind velocity is the velocity at the default reference height of 10 meters.
- The translation and rotation of the sails, and, optionally, the velocity of the motion.
- The local wing angles and the internal states of each sail. Without a controller, these are used in every time step. With a controller, they are only used as initial values, and the sails are controlled by the controller after the first time step. The `controller_loading` input sets the loading of the controller.

The outputs are the total forces and moments, the forces and moments on each sail, the angle of attack, velocity and apparent wind direction measured on each sail, and an error code (see below). The FMU supports up to 10 sails, which is the number of inputs and outputs for each sail.

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
| 12   | The simulation gave forces or moments that are not finite                     |
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
