# Change log pystormflow

This file contains an overview of the changes made to pystormflow, the Python interface to the Stormflow CFD solver. See the [Stormflow change log](../../stormflow/CHANGELOG.md) for changes to the solver, and the [Stormbird change log](../../stormbird/CHANGELOG.md) for changes to the actuator line model.

## 0.2.0 - 2026-10-23
### Breaking changes
- Depends on Stormflow 0.2.0, Stormbird 0.11.0 and Stormath 0.4.0. The input format for the Stormbird models has changed in Stormbird 0.11.0, so setup files may need to be updated.
- The `gyroscopic` property of the `SectionalForces` class is renamed to `gyroscopic_moment`, as the gyroscopic effect is now a pure moment in Stormbird. The `gyroscopic` property of the `IntegratedValues` class is unchanged.

## 0.1.0
The first version.
