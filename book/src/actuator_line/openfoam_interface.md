# OpenFOAM interface

The actuator line functionality in the core Stormbird library is written in a general way, and could be coupled to any CFD solver. However, at the moment, the only interface made is towards the open-source CFD solver [OpenFOAM](https://openfoam.com/). This chapter describes how the coupling works in practice, and what is necessary to set up a simulation using OpenFOAM together with Stormbird.

## CFD interfaces

The general source code for interfaces towards CFD solver is located in the [cfd_interfaces folder](https://github.com/NTNU-IMT/stormbird/tree/main/interfaces/cfd_interfaces) on Github. OpenFOAM is a c++ library, which is also a common language for other CFD codes. In an attempt to generalize as much as possible, the interface between Stormbird and OpenFOAM is divided in two parts.

The first part is a general C++ interface to the actuator line functionality in Stormbird. This source code is found in the [cpp_actuator_line](https://github.com/NTNU-IMT/stormbird/tree/main/interfaces/cfd_interfaces/cpp_actuator_line) folder on Github. It contains functionality to set up and call the Rust code from C++. The interface is made using the [cxx crate](https://cxx.rs/). It function names and data structures follows the Rust side as much as possible.

The second part is specific to OpenFOAM. This source code is found the [openfoam](https://github.com/NTNU-IMT/stormbird/tree/main/interfaces/cfd_interfaces/openfoam) folder on Github. It implements a class called `ActuatorLine` that inherits from the standard `cellSetOption` class in OpenFOAM. The `cellSetOption` base class is used to implement volume forces that are added to the momentum equations for a variety of functionalities in the OpenFOAM library. The connection logic behind this class and the other OpenFOAM solvers therefore follows the same principle as any other `fvOption` source in OpenFOAM.

The responsibility of the OpenFOAM interface is mainly to extract information from the CFD domain, such as the velocity field, pass it to the Stormbird library, and the project the resulting forces back to the CFD grid.

## Installing the OpenFOAM interface

Instructions for how to install the OpenFOAM interface can be found in the [README file](https://github.com/NTNU-IMT/stormbird/tree/main/interfaces/cfd_interfaces/openfoam) at the OpenFOAM interface Github page.

## Setting up an OpenFOAM simulation

Examples of how to use the OpenFOAM interface can be found in the [examples folder](https://github.com/NTNU-IMT/stormbird/tree/main/interfaces/cfd_interfaces/openfoam/examples) on GitHub.

As with any other `fvOption` model in OpenFOAM, the actuator line model is activated by specifying it in the `fvOptions` file in the `system` folder. However, unlike native OpenFOAM models, the general setup is not defined directly in the `fvOption` file. Rather, the only thing that is necessary to specify in the `fvOptions` file is the following [^interface_note]:

[^interface_note]: The interface for the setup kept simple, but might also probably a bit too basic to cover all situations at the moment. It might therefore be extended with more options in the future, depending on what is needed.

```c++
FoamFile
{
    version    2.0;
    format    ascii;
    class     dictionary;
    object    fvOptions;
}

actuatorLine
{
    type actuatorLine;
    selectionMode   all;
    fields (U);
    name actuatorLine;
}
```

This activates the actuator line functionality. The `ActuatorLine` class will then look for a JSON input file in the `system` folder called `stormbird_actuator_line.json`. The content of this file is a JSON representation of the `ActuatorLineBuilder` structure. If the file does not exists, or contains invalid settings, the OpenFOAM simulation will crash. The error message from OpenFOAM is messy in general, but there should be instructions from the Rust side within the crash log, typically on the top, explaining what went wrong.

## Results

Results from the simulation will be placed in the `postProcessing` folder in the case directory, like other post-processing data in OpenFOAM. There will be two types of result files:

- **The first is a simple csv file with forces** as a function of time. This file will be called `stormbird_forces.csv`. The forces are written for every time step. The point of this file is to have a simple representation of the most important values from a simulation
- **The second is folder with full simulation result data**. How often this data is written is controlled by the `write_iterations_full_result` parameter in the [ActuatorLineBuilder](simulation_overview.md) structure. If this value is set 100, the full results will be written every 100 time step. The folder is called `stormbird_full_results` and will contain several JSON files with [SimulationResult](../line_model/force_calculations.md) data. This data is useful for looking more detailed into the results, such as the circulation distribution and the angles of attack on each line segment.
