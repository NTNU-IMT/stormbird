# Functional Mockup Unit

The lifting line functionality in Stormbird is available as a *Functional Mockup Unit*, which means that the functionality can be executed through the [Functional Mockup Interface standard](https://fmi-standard.org/). The actual interface is generated using the [fmu_from_struct](https://github.com/jarlekramer/fmu_from_struct) library, which is made by the same developer as Stormbird.

## How to execute simulations using the FMU-version
The FMU-version is currently made to support version 2 of the FMI-standard. This choice is made because the developers of Stormbird primarily uses the [Open Simulation Platform](https://opensimulationplatform.com/) for executing simulations, which currently only supports version 2. An FMU that supports version 3 of the FMI-standard is relatively straight forward to make, but will probably not be prioritized before the Open Simulation Platform is updated to the latest version. 

A simulation can be executed with any simulation platform that supports the FMI-standard. One example is the [command line interface](https://open-simulation-platform.github.io/cosim) from the Open Simulation Platform, or some Python interface to FMU's such as [FMPy](https://github.com/CATIA-Systems/FMPy) or [PyFMI](https://jmodelica.org/pyfmi/).

## What can the FMU-version do?
The FMI-interface inherently comes with significantly more limitations than a conventional API, such as the Python interface. For instance, there are limitations on what type of variables that can be passed to and from different FMUs, and there is a specific order to how functions are executed. Although there are many ways to work around these limitations, the FMU-version of Stormbird is, for simplicity sake, designed to only cover the most typical use cases for running dynamic lifting line simulations. That is, **it is not intended to be a direct alternative to the Python interface**, but rather a specialized way to use *some* of the functionality.

To be more specific, there are essentially two primary use cases for the Stormbird FMU:

1) **Coupling of Stormbird to a time-domain ship simulator**, such as [VeSim](https://www.sintef.no/en/software/vesim/). VeSim is a ship simulator that includes maneuvering and seakeeping models of ships. It can, for instance, simulate a ship moving in waves, including the effect of rudder action and control systems. The software is built around the FMI-standard to couple different sub-models together. Stormbird can, therefore, be one of many models in a ship-system simulations in the time-domain.
2) **Running sail simulations in hybrid experiments**. Hybrid experiments are experiments where part of the physics is measured experimentally, while other parts are simulated. In the specific case of wind-powered ships, the aerodynamic forces on the sails are simulated while the hydrodynamics are tested in a towing tank. [This article](https://www.sciencedirect.com/science/article/pii/S0029801821015213?via%3Dihub) explains more of how this is done at SINTEF Ocean. The Stormbird FMU was designed to fit well with the laboratory software used at SINTEF Ocean when doing hybrid tests, called [HLCC](https://www.sintef.no/programvare/hlcc/).

There are no direct coupling to VeSim or HLCC in the Stormbird FMU, but the choice of input and output variables was made, in part, based on what makes sense for these external software packages. That is, the design of the Stormbird FMU interface is not made in isolation.

## Variables

Any FMU is defined through it's parameters, input and output variables. For a direct listing of all available variables, please see the model description file, which is in the FMU. An overall description of variables are given below.

### Parameters
Due to limitation in the way the HLCC software loads FMUs, all parameters for the Stormbird lifting line is moved to separate JSON file. The Rust structure of the parmatertes, which are [deserialized using Serde](io_overview.md) looks like the following: 

```rust
struct Parameters {
    pub lifting_line_setup_file_path: String,
    pub wind_environment_setup_file_path: String,
    pub angles_in_degrees: bool,
    pub negative_z_is_up: bool,
    pub reverse_wind_direction: bool,
    pub reverse_translational_velocity: bool,
    pub non_dim_spanwise_measurement_position: f64,
    pub model_scale_factor: f64,
    pub input_moving_average_window_size: usize,
    pub translational_velocity_in_body_fixed_frame: bool,
    pub max_input_velocity: Option<f64>,
    pub max_position_change_velocity: Option<f64>,
    pub max_rotation_change_velocity: Option<f64>,
}
```

Further explanation are given below:

- **lifting_line_setup_file_path**: A `String` giving a path to a JSON file that contain a full setup of a Stormbird lifting line simulation. The JSON file must contain fields that can be used to construct a [simulation builder](./../lifting_line/simulation_overview.md). The parameter is mandatory, as no simulation model can be created without this information.
- **wind_environment_setup_file_path**: An optional `String` that gives a path to a JSON file with input that can be used to construct a wind environment, e.g., a varying wind field as a function of height. **More to come on this!**
- **angles_in_degrees**: A `Boolean` tha specifies whether the angles given is in degrees or not. Default is `False`. When Stormbird is used together with VeSim, it should be set to `True`
- **negative_z_is_up**: A `Boolean` variable used to determine the direction of the z-axis. If the z-axis is positive downwards - which is often is in maneuvering simulations - this variable must be set to `True`. Default is `False`
- **reverse_wind_direction**: A `Boolean` that can be used to reverse the angle definition of the wind input. 
- **reverse_translational_velocity**: A `Boolean` that can be used to reverse the input velocity given in the input variables before it is sent to the lifting line model.
- **non_dim_spanwise_measurement_position**: A non-dimensional number used to determine where measurements (see output variables for an overview) should happen along the wing span. Zero means the middle of thw wing, while +/-0.5 means the ends of the wings. 
- **model_scale_factor**: A variable specifically intended for hybrid testing. When this number is larger than zero, Froude scaling will be applied to both the input and output of the FMU. The input, e.g., velocity and motion will be scaled from the specified model scale to full-scale before they are sent further to the lifting line simulation. The output, e.g., forces and moments, from the lifting line simulation will then be scaled down from full-scale to model scale again.
- **input_moving_average_window_size**: An optional integer to specify that a moving average filter should be applied to the input values. This is for situations where there might be a lot of noise in the input.
- **translational_velocity_in_body_fixed_frame**: A `Boolean` that specifies the coordinate system of the input translational velocity. When `False`, the values are assumed to be in a global coordinate system. When `True`, the translational velocity is assumed to be in a body fixed coordinate system.
- **max_values**. The variables `max_input_velocity`, `max_position_change_velocity`, and `max_rotation_change_velocity` all specify optional maximum values for the input. These are meant for dealing with situations where there might be sudden changes in the input due to noise measurements. They should only be used for hybrid testing situations, not time domain ship simulations.

### Inputs
The inputs are variables that are set, potentially based on output from other FMUs, right before a simulation step is executed. The available input is listed below:

- **translational_velocity_x**, **translational_velocity_y**, and **translational_velocity_z**: Three `Float`'s than be used to apply a translational velocity to the simulation. This can for instance be velocity due to the ship motion. **WARNING**: one should generally *either* apply translational velocity or a dynamic position. It will seldom make sense to apply both. That is, either the sails are assumed to stand still, but with varying translational velocity due to the ship motion, or the sails will move dynamically with updates to the x, y, and z, position which will further cause motion induced velocities in the model. Applying both will cause the motion induced velocities to be twice as large as it should be.
- **wind_velocity**: A `Float` that specifies the global true wind velocity
- **wind_direction_coming_from** A `Float` that specifies the global true wind direction, defined as where the wind is coming from.
- **x_position**, **y_position**, and **z_position**: Three `Float`'s that can be used to move the line force model in the x-, y- and z-direction in a global coordinate system. **WARNING**: should not be used together with the translational velocity variables. See above for more. 
- **x_rotation**, **y_rotation**, and **z_rotation**: Three `Float`'s that can be used to apply a rotation along the x-, y-, and z-axis relative to the initial geometry definition
- **local_wing_angle_1 - local_wing_angle_10**: Several `Float`'s, with local number between 1-10, that can be used to pass a local wing angles for the sails. This can be used, for instance, to maintain or change the angle of attack as a function of wind direction.
- **section_models_internal_state_1 - section_models_internal_state_10**: Several `Float`'s, with local number between 1-10, that can be used to pass the internal state of the section models, for instance the rotational speed for a rotating cylinder, or the flap angle for a multi-element foil profile.

### Outputs
The output is the results generated by the FMU, which primarily consist of force and moment measurements. Whether the output is in the global or local coordinate system depends on the settings in the [line force model](../line_model/line_model_intro.md)

- **force_x**, **force_y**, and **force_z**: Three `Float`'s that gives the total force on the line force model in the x-, y-, and z-direction. 
- **moment_x**, **moment_y**, and **moment_z**: Three `Float`'s that gives the total moment on the line force model in the x-, y-, and z-direction. 
- **estimated_apparent_wind_direction**: A measured value for the apparent wind direction on the sails. The measurement are taken at the `non_dim_spanwise_measurement_position` parameter for each sail, and then averaged over all sails in the model. The intended use case is to pass it to a control system that may need an estimate of the wind direction to determine the right wing angle and sectional model internal state.
- **angle_of_attack_measurement_1 - angle_of_attack_measurement_10**: Several `Float`'s, with local number between 1-10, that measure the felt angle of attack on each wing. The intended use case is to pass this estimate to a control system FMU that may need information about the local angle of attack to determine an update to the local wing angles.

## Coordinate systems
In both VeSim, and many other rigid-body simulators, it is practical to have forces in a body fixed coordinate system. The initial sail-geometry and resulting forces are therefore assumed to be defined in a body fixed coordinate system. **Note:** for this to be correct, the line force model must be configured to give force output in a body-fixed coordinate system. See the [line model chapter](./../line_model/line_model_intro.md) for more on this. The motion of the sails and the wind conditions are assumed to be defined in a global coordinate system, as this is more straightforward to set up when the ship can move freely. 

In VeSim, and many other ship maneuvering simulators, the convention is to define body fixed coordinate systems such that the x-axis points forward, the y-axis points to the starboard side, and the z-axis point downwards. The Stormbird FMU does not assume this directly, but contains variables that can be manipulated to support different coordinate system conventions. 



