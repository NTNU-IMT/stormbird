# Functional Mockup Unit

The lifting line functionality in Stormbird is available as a *Functional Mockup Unit*, which means that the functionality can be executed through the [Functional Mockup Interface standard](https://fmi-standard.org/). The actual interface is generated using the [fmu_from_struct](https://github.com/jarlekramer/fmu_from_struct) library, which is made by the same developer as Stormbird.

## How to execute simulations using the FMU-version
The FMU-version is currently made to support version 2 of the FMI-standard. This choice is made because the developers of Stormbird primarily uses the [Open Simulation Platform](https://opensimulationplatform.com/) for executing simulations, which currently only supports version 2. An FMU that supports version 3 of the FMI-standard is relatively straight forward to make, but will probably not be prioritized before the Open Simulation Platform is updated to the latest version.

A simulation can be executed with any simulation platform that supports the FMI-standard. One example is the [command line interface](https://open-simulation-platform.github.io/cosim) from the Open Simulation Platform, or some Python interface to FMU's such as [FMPy](https://github.com/CATIA-Systems/FMPy) or [PyFMI](https://jmodelica.org/pyfmi/).

To actually set up a simulation, it is necessary to pass in several parameter- and input variables to the FMU unit. For a full overview of the available variables, see the [FMU source code](https://github.com/NTNU-IMT/stormbird/tree/main/interfaces/fmus/stormbird_lifting_line). To see how it can be used in practice, see the [FMU example folder](https://github.com/NTNU-IMT/stormbird/tree/main/interfaces/fmus/examples).

## What can the FMU-version do?
The FMI-interface inherently comes with more limitations than a conventional API, such as the Python interface. For instance, there are limitations on what type of variables that can be passed to and from different FMUs, and there is a specific order to how functions are executed. Although there are many ways to work around these limitations, the FMU-version of Stormbird is, for simplicity sake, designed to only cover the most typical use cases for running dynamic lifting line simulations. That is, **it is not intended to be a direct alternative to the Python interface**, but rather a specialized way to use *some* of the functionality.

To be more specific, there are essentially two primary use cases for the Stormbird FMU:

1) **Coupling of Stormbird to a time-domain ship simulator**, such as [VeSim](https://www.sintef.no/en/software/vesim/). VeSim is a ship simulator that includes maneuvering and seakeeping models of ships. It can, for instance, simulate a ship moving in waves, including the effect of rudder action and control systems. The software is built around the FMI-standard to couple different sub-models together. Stormbird can, therefore, be one of many models in a ship-system simulations in the time-domain.
2) **Running sail simulations in hybrid experiments**. Hybrid experiments are experiments where part of the physics is measured experimentally, while other parts are simulated. In the specific case of wind-powered ships, the aerodynamic forces on the sails are simulated while the hydrodynamics are tested in a towing tank. [This article](https://www.sciencedirect.com/science/article/pii/S0029801821015213?via%3Dihub) explains more of how this is done at SINTEF Ocean. The Stormbird FMU was designed to fit well with the laboratory software used at SINTEF Ocean when doing hybrid tests, called [HLCC](https://www.sintef.no/programvare/hlcc/).

There are no direct coupling to VeSim or HLCC in the Stormbird FMU, but the choice of input and output variables was made, in part, based on what makes sense for these external software packages. That is, the design of the Stormbird FMU interface is not made in isolation.
