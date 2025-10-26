# OpenFOAM version

[OpenFOAM](https://www.openfoam.com/) is a general open source CFD solver, widely used both in academia and in industry.

The OpenFOAM version of Stormbird is currently the only way to run actuator line simulations. It consist of a volume force interface between the Stormbird library and the OpenFOAM library, that can be activated together with a solver in OpenFOAM.
More details about this interface can be found both in the [actuator line chapter](../actuator_line/actuator_line.md) and the specific chapter about the [OpenFOAM interface](../actuator_line/openfoam_interface.md).

As a general not, this interface could also serve as an inspiration for how to connect Stormbird to other CFD solvers, if this turns out to be relevant in the future.
