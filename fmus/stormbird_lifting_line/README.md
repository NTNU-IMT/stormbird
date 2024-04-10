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

## Model overview
### Parameters
- `setup_file_path`: A path to a Stormbird set-up file that generates the actual model with initial 
conditions for the geometry. The reason for not exposing the full setup directly in the 
`modelDescription.xml` file is to be able to use the same setup functionality as the rest of 
Stormbird. That is, Stormbird already uses the `serde` crate to automatically serialize and 
deserialize data structures, and the setup of a Stormbird model is therefore done using JSON files 
as default. Rewriting an interface to do the same task with input from an FMI file is more work and 
seems unnecessary from a user perspective (can be discussed).
### Input
- `rotation_x`, `rotation_y`, and `rotation_z`: Rotation vector. The same rotation is applied to all sails 
(the relative position remains the same). The origin of the rotation is assumed to be the translated 
origo in the initial coordinate system. That is, the rotation center is translated along with the 
sails
- `translation_x`, `translation_y`, and `translation_z`: Translation vector. As for the rotation, the same 
translation is applied to all sails. The translation is relative to the initial configuration.
- `freestream_u`, `freestream_v`, and `freestream_w`: Free stream velocity vector.
- `force_x`, `force_y`, and `force_z`: Integrated and summed up forces in x, y and z direction
- `moment_x`, `moment_y`, and `moment_z`: Integrated and summed up moments in x, y, and z direction

