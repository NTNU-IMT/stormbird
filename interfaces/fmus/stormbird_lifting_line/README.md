# Functional Mockup Interface (FMI) to run lifting line simulations

This crate generates a Functional Mockup Unit (FMU) that can execute lifting line simulations in a
dynamic situation, possibly together with other models that support the
[FMI-standard](https://fmi-standard.org/).

## Build instructions
The interface is automatically generated using the `FmrsModel` derive macro. Build instructions are
as follows:

- Run `cargo build`. This will compile the lifting line model with the interface and generate a
model description file.
- Alternative: First run `set RUSTFLAGS="-C target-cpu=native"`, then `cargo run --release` to
optimize specifically for the local CPU.
- Assuming the Python module `fmrs_build_utils` is installed, run
`python -m fmrs_build_utils.package_fmu`. This will take the result from the compilation
process and package it into an FMU file.
- Copy/move the resulting `StormbirdLiftingLine.fmu` to wherever you like, and load it using your
preferred FMI simulator to execute. This can for instance be
[FMPy](https://github.com/CATIA-Systems/FMPy) or the simulator from
[Open Simulation Platform](https://opensimulationplatform.com/)
