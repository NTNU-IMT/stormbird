# A C++ interface to the actuator line implementation in stormbird
This library allows for various interfaces to the actuator line implementation in Stormbird.

The use case is for implementing an interface to the actuator line functionality for a CFD solver written in C++.

## Build instructions
The library is compiled using cargo. The command line utility `cxxbridge` is also used to generate header files for a cpp compiler. 

To install cxxbridge run:

```
cargo install cxxbridge-cmd
```

The build the library and copy the resuklt over to OpenFOMA by running `bash build.sh` i


