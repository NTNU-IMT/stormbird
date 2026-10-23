# Change log pystormbird

This file contains an overview of the changes made to pystormbird, the Python interface to the Stormbird library. The version number of pystormbird follows the version number of the Stormbird Rust library. See the [Stormbird change log](../../stormbird/CHANGELOG.md) for changes to the underlying models.

## 0.11.0 - 2026-10-23
- Depends on the published versions of Stormbird 0.11.0 and Stormath 0.4.0, rather than the local versions in the repository.
- New `ActiveAirFlowFoil` class, including functions for the section internal states and the power for a whole wing, and a new `power_coefficient` function for the `RotatingCylinder` class. The `as_active_air_flow_foil` function on the `VaryingFoil` class was removed, as an active air flow foil now also needs an internal state conversion.
- New `set_internal_state_linear` and `set_internal_state_piecewise_constant` functions for the lifting line simulation and the complete sail model, for setting internal states that vary along the span. The `internal_state` functions return the mean value over the span for wings with a span varying internal state.
- The `Foil` class was updated with the new and removed fields in the foil model.
- The `RotatingCylinder` class was updated with the new lift parameters.
- The `Foil` class has getters and setters for all fields, and the `RotatingCylinder` class has getters and setters for all fields, together with `__str__` and `__dict__`.
- Invalid input when creating section models, simulations, line force models, complete sail models and wind environments now raises a `ValueError` rather than a panic. Reading simulation results from file raises an `OSError` if the file can not be read, and a `ValueError` if the content can not be parsed.
- New `wake_as_vtk` function on the lifting line simulation, which works for both quasi-steady and dynamic wakes, and `induced_velocities` now also works for quasi-steady wakes. `dynamic_wake_as_vtk` is deprecated, gives a `DeprecationWarning`, and returns the same data as `wake_as_vtk`.
- The rotor sail and dynamic wake shape examples now write the wake files from the script, with `wake_as_vtk`.
- The getters and setters for `cdi_correction_factor`, `added_mass_factor` and `moment_of_inertia_2d` were removed from the `Foil` and `RotatingCylinder` classes. The `gyroscopic` property of the `SectionalForces` class is renamed to `gyroscopic_moment`.

## 0.10.0 and earlier
Changes up to and including version 0.10.0 were mainly made to keep pystormbird in sync with the Stormbird Rust library. As models are set up with JSON strings, most changes to the input format in the Rust library also affect the Python side. See the [Stormbird change log](../../stormbird/CHANGELOG.md) for details.
