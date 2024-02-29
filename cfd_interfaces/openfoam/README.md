# OpenFOAM interface to the Stormbird actuator line functionality

## Install instructions
### Dependencies
- Rust
- OpenFOAM
- cxxbridge-cmd, a rust crate for managing cxx-libraries. Run `cargo install cxxbridge-cmd` to install.

### How to install
- Make sure a development version of OpenFOAM is loaded in the terminal. The build system `wmake` must be available.
- Rust and cxxbridge-cmd must also be available, but this should be the case if they are installed in the normal way.
- Run the build script named `build.sh`