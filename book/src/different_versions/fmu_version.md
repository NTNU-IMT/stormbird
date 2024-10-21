# Functional Mockup Unit

The lifting line functionality in Stormbird is available as a *Functional Mockup Unit*, which means that the functionality can be executed through the [Functional Mockup Interface standard](https://fmi-standard.org/). The actual interface is generated using the [fmu_from_struct](https://github.com/jarlekramer/fmu_from_struct) library, which is made by the same developer as Stormbird.

## How to execute simulations using the FMU-version
The FMU-version is currently made to support version 2 of the FMI-standard. This choice is made because the developers of Stormbird primarily uses the [Open Simulation Platform](https://opensimulationplatform.com/) for executing simulations, which currently only supports version 2. An FMU that supports version 3 of the FMI-standard is relatively straight forward to make, but will probably not be prioritized before the Open Simulation Platform is updated to the latest version. 

A simulation can be executed with any simulation platform that supports the FMI-standard. One example is the [command line interface](https://open-simulation-platform.github.io/cosim) from the Open Simulation Platform.

## What can the FMU-version do?
The FMI-interface inherently comes with significantly more limitations than a conventional API, such as the Python interface. For instance, there are limitations on what type of variables that can be passed to and from different FMUs, and there is a specific order to which functions that are executed when. Although there are many ways to work around these limitations, the FMU-version of Stormbird is, for simplicity sake, designed to only cover the most typical use cases where an FMU version might be preferable over a Python interface. That is, **it is not intended to be a direct alternative to the Python interface**, but rather a specialized way to use *some* of the functionality.

The Stormbird FMU was made to allow sails to be simulated in the time domain together with models of the rest of the ship, for instance to do maneuvering or seakeeping simulations. In particular, the FMU version of Stormbird is made specifically to be suitable as a co-simulator for the [VeSim](https://www.sintef.no/en/software/vesim/) ship simulator. There are no direct coupling to VeSim, but the choice of input and output variables was made, in part, based on what is needed when running VeSim simulations.

In other words: the FMU version of Stormbird assumes that there are sails placed on a ship, and that the ship is, potentially, moving in six degrees of freedom. The overall goal is to compute forces and moments as output, that can be passed to another FMU that can use them to compute how the ship is affected by the sails.

## Variables

Any FMU is defined through it's parameters, input and output variables. For a direct listing of all available variables, please see the model description file, which is in the FMU. An overall description of variables are given below.

### Parameters
Parameters are used to set up the model before the simulation starts. The available parameters are listed below:

- **lifting_line_setup_file_path**: A `String` giving a path to a JSON file that contain a full setup of a Stormbird lifting line simulation. The JSON file must contain fields that can be used to construct a [simulation builder](./../lifting_line/simulation_overview.md). The parameter is mandatory, as no simulation model can be created without this information.
- **wind_environment_setup_file_path**: An optional `String` that gives a path to a JSON file with input that can be used to construct a wind environment, e.g., a varying wind field as a function of height. **More to come on this!**
- **angles_in_degrees**: A `Boolean` tha specifies whether the angles given is in degrees or not. Default is `False`. When Stormbird is used together with VeSim, it should be set to `True`
- **relative_wind_direction_measurement_non_dim_height**: An optional `Float` variable that determines where the relative wind direction measurements are taken (see the output variables for an explanation of what this means). The value should be between 0 and 1, and specifies where along the span the measurements should be taken. 
- **include_induced_velocities_in_wind_direction_measurements**: A `Boolean` variable that determines whether induced velocities are included in the wind angle measurements
- **negative_z_is_up**: A `Boolean` variable used to determine the direction of the z-axis. If the z-axis is positive downwards - which is often is in maneuvering simulations - this variable must be set to `True`. Default is `False`
- **reverse_wind_direction**: A `Boolean` that can be used to reverse the angle definition of the wind input. 
- **export_stormbird_result**: A `Boolean` variable that determines whether the full result structure from Stormbird should be exported as a string in the output. 

### Inputs
The inputs are variables that are set, potentially based on output from other FMUs, right before a simulation step is executed. The available input is listed below:

- **wind_velocity**: A `Float` that specifies the global true wind velocity
- **wind_direction_coming_from** A `Float` that specifies the global true wind direction, defined as where the wind is coming from.
- **x_position**, **y_position**, and **z_position**: Three `Float`'s that can be used to move the line force model in the x-, y- and z-direction in a global coordinate system
- **x_rotation**, **y_rotation**, and **z_rotation**: Three `Float`'s that can be used to apply a rotation along the x-, y-, and z-axis relative to the initial geometry definition
- **local_wing_angles**: A `String` that can be used to pass a list of local wing angles.

### Outputs
The output is the results generated by the FMU, which primarily consist of force and moment measurements. Whether the output is in the global or local coordinate system depends on the settings in the [line force model](../line_model/line_model_intro.md)

- **force_x**, **force_y**, and **force_z**: Three `Float`'s that gives the total force on the line force model in the x-, y-, and z-direction. 
- **moment_x**, **moment_y**, and **moment_z**: Three `Float`'s that gives the total moment on the line force model in the x-, y-, and z-direction. 
- **relative_wind_direction_measurements**: A string containing a list with the measured wind direction on each sail in the body fixed coordinate system. Meant to be sent to a sail controller.
- **stormbird_result**: If activated in the input, this variable gives the full Stormbird output as a JSON string


## Coordinate systems
In both VeSim, and many other rigid-body simulators, it is practical to have forces in a body fixed coordinate system. The initial sail-geometry and resulting forces are therefore assumed to be defined in a body fixed coordinate system. **Note:** for this to be correct, the line force model must be configured to give force output in a body-fixed coordinate system. See the [line model chapter](./../line_model/line_model_intro.md) for more on this. The motion of the sails and the wind conditions are assumed to be defined in a global coordinate system, as this is more straightforward to set up when the ship can move freely. 

In VeSim, and many other ship maneuvering simulators, the convention is to define body fixed coordinate systems such that the x-axis points forward, the y-axis points to the starboard side, and the z-axis point downwards. The Stormbird FMU does not assume this directly, but contains variables that can be manipulated to support different coordinate system conventions. 



