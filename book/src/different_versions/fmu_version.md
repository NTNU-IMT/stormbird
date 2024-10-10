# Functional Mockup Unit

The lifting line functionality in Stormbird is available as a *Functional Mockup Unit*, which means that the functionality can be executed through the [Functional Mockup Interface standard](https://fmi-standard.org/). The actual interface is generated using the [fmu_from_struct](https://github.com/jarlekramer/fmu_from_struct) library, which is made by the same developer as Stormbird.

## How to execute simulations using the FMU-version
The FMU-version is currently made to support version 2 of the FMI-standard. This choice is made because the developers of Stormbird primarily uses the [Open Simulation Platform](https://opensimulationplatform.com/) for executing simulations, which currently only supports version 2. An FMU that supports version 3 of the FMI-standard is relatively straight forward to make, but will probably not be prioritized before the Open Simulation Platform is updated to the latest version. 

A simulation can be executed with any simulation platform that supports the FMI-standard. One example is the [command line interface](https://open-simulation-platform.github.io/cosim) from the Open Simulation Platform.

## What can the FMU-version do?
The FMI-standard is not meant to be a general interface to any type of software and functionality, but rather a standard for how models can be connected in the specific use case of performing co-simulations in the time domain. As such, the FMI-interface inherently comes with significantly more limitations than a conventional API, such as the Python interface. For instance, there are limitations on what type of variables that can be passed to and from different FMUs, and there is a specific order to how functions are executed and which functions are executed. To avoid complicating things too much, the FMU-version of Stormbird is designed to only cover the most typical use cases where an FMU version might be preferable over a Python interface. That is, it is not intended to be a direct alternative to the Python interface, but rather a specialized way to use *some* of the functionality.

The most obvious use case for an FMU version is when the goal is to run sail simulations together with other models representing the ship that also supports the FMU-standard, for instance to include hydrodynamic models, engine and system models, and control systems. The Stormbird FMU was made to allow sails to be simulated in the time domain together with the rest of the ship, for instance to do maneuvering or seakeeping simulations. In particular, the FMU version of Stormbird is made specifically to be suitable as a co-simulator for the [VeSim](https://www.sintef.no/en/software/vesim/) ship simulator. There are no direct coupling to VeSim, but the choice of input and output variables was made, in part, based on what is needed when running VeSim simulations.

In other words: the FMU version of Stormbird assumes that there are sails placed on a ship, and that the ship is, potentially, moving in six degrees of freedom. The overall goal is to compute forces and moments as output, that can be passed to another FMU that can use them to compute how the ship is affected by the sails.

## Variables

## Coordinate systems
In both VeSim, and many other rigid-body simulators, it is practical to have forces in a body fixed coordinate system. The sail-geometry and resulting forces are therefore assumed to be defined in a body fixed coordinate system. However, the wind conditions are assumed to be defined in a global coordinate system, as this is more straightforward to set up when the ship can move freely. 

In VeSim, and many other ship maneuvering simulators, the convention is to define body fixed coordinate systems such that the x-axis points forward, the y-axis points to the starboard side, and the z-axis point downwards. The Stormbird FMU does not assume this directly, but contains variable that can be manipulated to support different coordinate system conventions. The default is a conventional maneuvering coordinate system.



