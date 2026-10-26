# Change log pystormflow

This file contains an overview of the changes made to pystormflow, the Python interface to the Stormflow CFD solver. See the [Stormflow change log](../../stormflow/CHANGELOG.md) for changes to the solver, and the [Stormbird change log](../../stormbird/CHANGELOG.md) for changes to the actuator line model.

## 0.3.0 - unreleased
### Breaking changes
- Depends on Stormflow 0.3.0, where the `slip_wall_boundary_override` setting is replaced by the new `boundary_condition_type` setting, with the variants `WindInletOutlet` (default, as before) and `WindTunnel`. See the [Stormflow change log](../../stormflow/CHANGELOG.md).

## 0.2.1
- Depends on Stormflow 0.2.1 and Stormbird 0.11.1. The main changes are new experimental limits on the lifting line correction in the actuator line model. See the [Stormflow change log](../../stormflow/CHANGELOG.md) and the [Stormbird change log](../../stormbird/CHANGELOG.md).

## 0.2.0 - 2026-10-23
### Breaking changes
- Depends on Stormflow 0.2.0, Stormbird 0.11.0 and Stormath 0.4.0. The input format for the Stormbird models has changed in Stormbird 0.11.0, so setup files may need to be updated.
- The `gyroscopic` property of the `SectionalForces` class is renamed to `gyroscopic_moment`, as the gyroscopic effect is now a pure moment in Stormbird. The `gyroscopic` property of the `IntegratedValues` class is unchanged.

## 0.1.0
The first version.
