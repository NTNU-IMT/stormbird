# A C++ interface to the actuator line implementation in stormbird
This crate is a general C++ interface to the actuator line functionality in Stormbird. The intended use case is to allow Stormbird to be connected to CFD libraries written in C++.

## Build instructions
The library is compiled using cargo. The command line utility `cxxbridge` is also used to generate header files for a cpp compiler.

To install cxxbridge run:

```
cargo install cxxbridge-cmd
```

To build the library run:

```
cargo build --release
```
