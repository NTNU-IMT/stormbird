# Simulations with Stormbird using FMUs

This folder contains *Functional Mockup Units* (FMUs) that use the Stormbird library as it's base. The *Functional Mockup Interface* (FMI) is generated using the [fmu_from_struct](https://github.com/jarlekramer/fmu_from_struct) macro. Each FMU is built and packaged with the `cargo fmu-from-struct` subcommand, see the README of each FMU.

## Folder structure
- `stormbird_lifting_line` is a FMU that runs the lifting line simulation in Stormbird.
- `utils` contains simple FMUs that are useful when setting up tests and examples, such as FMUs that give prescribed motions.
- `examples` contains examples of how to use the Stormbird FMUs in practical simulations.
