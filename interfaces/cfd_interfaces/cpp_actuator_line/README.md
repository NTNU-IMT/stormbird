# A C++ interface to the actuator line implementation in stormbird
This crate is a general C++ interface to the actuator line functionality in Stormbird. The intended use case is to allow Stormbird to be connected to CFD libraries written in C++.

## Build instructions
The library can be compiled using cargo. The command line utility `cxxbridge` is also used to generate header files for a cpp compiler.

To install cxxbridge run:

```
cargo install cxxbridge-cmd
```

To build the library run:

```
cargo build --release
```

This gives a static library, `target/release/libcpp_actuator_line.a` on Linux. The library uses the versions of Stormbird and Stormath that are published on [crates.io](https://crates.io/crates/stormbird), as given in `Cargo.toml`.

The C++ header and source files for the interface are generated from the Rust code with cxxbridge:

```
cxxbridge src/lib.rs --header > cpp_actuator_line.hpp
cxxbridge src/lib.rs > cpp_actuator_line.cpp
```

The generated source file must be compiled together with the C++ code that uses the interface, and the static library must be linked. See the build script in the [OpenFOAM interface](../openfoam/build.sh) for an example.
