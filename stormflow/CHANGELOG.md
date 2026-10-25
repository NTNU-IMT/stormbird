# Change log Stormflow

This file contains an overview of the changes made to Stormflow, the CFD solver specialized for actuator line simulations with Stormbird.

## 0.2.0 - 2026-10-25
- Updated to stormbird v0.11.1, which is a small change. The main feature is new experimental limits on the lifting line correction (see the stormbird release notes for more)

## 0.2.0 - 2026-10-23
### Breaking changes
- Stormflow now depends on version 0.11.0 of Stormbird and version 0.4.0 of Stormath. The input format for the Stormbird models has changed in Stormbird 0.11.0, so input files may need to be updated. See the [Stormbird change log](../stormbird/CHANGELOG.md) for details. Types from Stormbird and Stormath that are part of the public interface of Stormflow, such as `ActuatorLine` and `SpatialVector`, now come from the new versions.
- The minimum supported Rust version is set to 1.88, through the new `rust-version` field.

### Bug fixes
- The GPU version of the turbulence model computed a wrong eddy viscosity in a small number of cells, up to about 20% off in this test case, as the velocity gradient in the shader was miscompiled on at least some GPU backends (seen on Vulkan with an NVIDIA GPU). The shader built the gradient tensor by writing to the vector components with dynamic indices inside nested loops, and the rotation part of the gradient was partly lost. The tensor, and the strain rate tensor computed from it, are now built with constructors instead, and the GPU version gives the same results as the CPU version within floating point accuracy. This made the tests that compare the GPU and CPU versions of the turbulence solver fail.

### Other changes
- The command line option for the internal state of the wings is renamed from `--section-models-internal-state` to `--wing-internal-states`, as it sets the internal state of each wing, e.g., the revolutions per second of a rotor sail. The short name, `-s`, and the old long name still work. The option now accepts several values after a single flag, e.g., `-s -1.273 -1.273`, and a single value is used for all wings. Giving a number of values that does not match the number of wings, or giving values when the setup has no actuator line model, is now an error, rather than a crash or a silent omission.
- Removed the `revolutions_per_second` field from the rotor sail examples, as the rotational speed is no longer part of the section model in Stormbird, and can not be given in the setup file. The rotational speed is instead set with the `-s` option, as described in the new usage section in the README.
- New usage section in the README, which describes the command line options.
- The smoke tests that use the rotor sail example set the rotational speed of the rotor after the simulation is built, as it is no longer part of the setup file.
- The single wing and SOBC1 examples are updated to the current input format, as they could not be read in version 0.1.0 either. The grid is now given in the `grid` field, with the same domain and cell size as before, the section models use the `SingleElementFoil` variant, and the controller in the SOBC1 example uses the `spanwise_measurement` field to measure the flow conditions over the middle of the wing.
- New test that checks that all the example setup files can be read.

## 0.1.0
The current version. Stormflow was added to the repository together with version 0.9.0 of Stormbird, and was updated substantially together with version 0.10.0 of Stormbird, to make it ready for the first "official" version. Changes up to this version were mainly made to develop the solver and to keep it in sync with the Stormbird Rust library.
