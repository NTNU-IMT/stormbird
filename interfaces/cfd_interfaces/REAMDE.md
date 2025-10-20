# CFD interfaces to Stormbird

CFD interfaces are used to execute the actuator line functionality within a CFD solver. At the moment, there are two interfaces:

- A general C++ interface to the actuator line functionality, located in the [cpp_actuator_line](/cpp_actuator_line) folder. This interface is necessary as many CFD solvers are written in C++. The idea behind this interface is to be independent of the exact solver, and be a direct interface to the underlying Rust code.
- A specific interface to [OpenFOAM](https://www.openfoam.com/) located in the [openfoam](/openfoam) folder
