# Simulations with Stormbird using FMUs

This folder contains *Functional Mockup Units* (FMUs) that use the Stormbird library as it's base. The *Functional Mockup Interface* (FMI) is generated using the [fmu_from_struct](https://github.com/jarlekramer/fmu_from_struct) macro.

## Folder structure
- `stormbird_lifting_line` is a FMU that runs the lifting line simulation in Stormbird.
- `wind_environment` contains a FMU that generate a spatial varying wind field, based on a model of the atmospherics boundary layer, and, optionally, a constant velocity to represent, for instance, the felt velocity due to the ship speed.
- `vesim_utils` contains an FMUs that is useful when running the Stormbird lifting line FMU together wih the [VeSim](https://www.sintef.no/programvare/vesim/) ship FMU. In particular, this folder contains FMUs that translates output from one FMU to the right input to another FMU.
- `examples` contains examples of how to use the Stormbird FMUs in practical simulations.