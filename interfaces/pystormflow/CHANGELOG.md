# Change log pystormflow

This file contains an overview of the changes made to pystormflow, the Python interface to the Stormflow CFD solver. See the [Stormflow change log](../../stormflow/CHANGELOG.md) for changes to the solver, and the [Stormbird change log](../../stormbird/CHANGELOG.md) for changes to the actuator line model.

## Unreleased
### Features
- New `Simulation.export_correction_wake_as_vtk(file_path, binary)` and `Simulation.correction_wake_as_vtk(binary)`, to export the wake of the lifting line correction as a legacy VTK file, e.g., to follow the shape of the new dynamic wake during a simulation. The export returns `False` if there is no wake to export.
- The new `Dynamic` wake of the lifting line correction is supported, with the velocity at the wake points sampled by Stormflow. See the Stormflow and Stormbird change logs.

### Changes
- Depends on the local versions of Stormflow, Stormbird and Stormath in this repository (with the version numbers as before), as the new features are not published yet.
- From Stormbird 0.14.0: `remove_span_velocity` in the sampling settings of the actuator line is true by default, and the angle of attack no longer depends on the velocity along the span. See the Stormbird change log.

## 0.4.1 - 2026-11-20
### Changes
- Depends on the published versions of Stormflow 0.4.1 and Stormbird 0.13.1, so that setup files can use the new `internal_states` setting in the `LineForceModelBuilder`, which sets the initial internal state of each wing. There are no changes to the Python interface itself. Setup files that are valid for pystormflow 0.4.0 give the same results, except for the bug fixes below.

### Bug fixes
- From Stormbird 0.13.1: the rate limit of the local wing angles in the controller of the actuator line model (`max_local_wing_angle_change_rate`) works correctly when the wing angle crosses pi. Before, the wings could be turned the wrong way and get stuck, typically with stalled sails.
- From Stormbird 0.13.1: suction sails with the `PowerToInternalFlow` conversion no longer panic for very small, but non-zero, powers.

## 0.4.0 - 2026-11-14
### Breaking changes
- Depends on the published versions of Stormflow 0.4.0, Stormbird 0.13.0 and Stormath 0.5.0. There are no changes to the Python interface itself. Setup files that are valid for pystormflow 0.3.0 give the same results. See the [Stormflow change log](../../stormflow/CHANGELOG.md) and the [Stormbird change log](../../stormbird/CHANGELOG.md) for the details.

### Other changes
- Settings that are found to be invalid when the simulation is built, e.g., an inconsistent wing geometry or an invalid controller in the actuator line model, raise a `ValueError` when the `Simulation` is created, with the error message from Stormbird or Stormflow. Before, these settings gave a panic.

## 0.3.0 - 2026-10-31
### Breaking changes
- Depends on the published versions of Stormflow 0.3.0 and Stormbird 0.12.0. There are no changes to the Python interface itself, but the setup files may need to be updated. See the [Stormflow change log](../../stormflow/CHANGELOG.md) for the changes to the solver, such as the new `boundary_condition_type` setting, and the [Stormbird change log](../../stormbird/CHANGELOG.md) for the changes to the actuator line model, such as the new dynamic stall model.

### Other changes
- Invalid input raises a Python exception rather than a panic, in the same way as in pystormbird: creating a `Simulation` from a setup string that can not be read raises a `ValueError`, and reading simulation results from file with `SimulationResult.from_json_file` or `SimulationResult.result_history_from_file` raises an `OSError` if the file can not be read, and a `ValueError` if the content can not be parsed. Settings that are found to be invalid when the simulation is built in Stormflow still give a panic.
- New `section_models_internal_state` and `amount_of_stall` properties on the `SectionalForcesInput` class, as in pystormbird.
- The source distribution is compiled with native CPU optimizations, through a new `.cargo/config.toml`, as in pystormbird.
- The single rotor example is replaced by the new `rotor_sails/deybach_wind_tunnel` example, with one or two rotor sails in a wind tunnel, compared against the experiments by Deybach et al.

## 0.2.1
- Depends on Stormflow 0.2.1 and Stormbird 0.11.1. The main changes are new experimental limits on the lifting line correction in the actuator line model. See the [Stormflow change log](../../stormflow/CHANGELOG.md) and the [Stormbird change log](../../stormbird/CHANGELOG.md).

## 0.2.0 - 2026-10-23
### Breaking changes
- Depends on Stormflow 0.2.0, Stormbird 0.11.0 and Stormath 0.4.0. The input format for the Stormbird models has changed in Stormbird 0.11.0, so setup files may need to be updated.
- The `gyroscopic` property of the `SectionalForces` class is renamed to `gyroscopic_moment`, as the gyroscopic effect is now a pure moment in Stormbird. The `gyroscopic` property of the `IntegratedValues` class is unchanged.

## 0.1.0
The first version.
