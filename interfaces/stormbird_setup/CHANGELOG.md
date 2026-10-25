# Change log stormbird_setup

This file contains an overview of the changes made to stormbird_setup, the Python library for setting up Stormbird models. See the [Stormbird change log](../../stormbird/CHANGELOG.md) for changes to the underlying models.

## 0.11.3 - 2026-10-24
### Features
- Disks are now supported as geometries in OpenFOAM cases, for instance as end disks on rotor sails. They are meshed as solid cylinders, so they must have a thickness larger than zero, and the fillet radius must be zero.
- The `LiftingLineCorrectionBuilder` has the new, experimental, settings `max_correction_ratio` and `virtual_extension_factors`. They require a version of Stormbird newer than 0.11.0, and are only included in the JSON output when they are set, so the output is unchanged for older versions.

### Bug fixes
- OpenFOAM cases failed when stormbird_setup was built on Windows, as the files in the base folder got CRLF line endings, which bash and OpenFOAM do not accept. `write_foam_case` now converts all text files in the case folder to LF line endings, and the repository enforces LF line endings with a `.gitattributes` file.
- The refinement around the actuator lines in OpenFOAM cases was skipped when the refinement zone was small relative to the background cells, as snappyHexMesh only refines the cells with their center inside the zone. The refinement cylinder is now extended by half the diagonal of a background cell, both in radius and at the ends, so that all cells that overlap the zone are refined.

## 0.11.2 - 2026-10-24
### Bug fixes (which failed, see version 0.11.3 instead)
- OpenFOAM cases failed when stormbird_setup was built on Windows, as the files in the base folder got CRLF line endings.Tried first updating just the `.gitattributes` file, but that did not work it seamed...

## 0.11.1 - 2026-10-24
### Highlights
- Added a basic OpenFOAM setup feature for scripting simulations with actuator lines

## 0.11.0 - 2026-10-23
### Highlights
- New `internal_compartment_boundaries` field in the `WingBuilder`, with the same checks as in the Rust library.
- New `SectionModel.suction_sail_turbosail_charrier_1985` model, which is a suction sail model with the `PowerToInternalFlow` conversion, tuned to the TurboSail wind tunnel data from Charrier et al. (1985). The values are generated with the suction sail example in pystormbird.
- Updated to the new data structures for the section models, input power models and controller set points, including the fan model with fan curves. The fan curve data is checked in the same way as in the Rust library. The input power model now matches the variants in the Rust library, and `new_polynomial_rotor_sail_model` gives the power directly, without the `area` argument. The suction sail in the simplified setup now uses the default input power model.
- The `Foil` class was updated with the new and removed fields in the foil model.
- The `RotatingCylinder` class was updated with the new lift parameters. The `rotor_sail_ostman_2023` and `rotor_sail_deybach_2024` models now use parameters fitted to the data in the respective papers.
- New `far_field_ratio` and `initial_relative_wake_length` fields in the `DynamicWakeBuilder`, with the same defaults as in the Rust library.
- The `VaryingFoil` class has the same checks as in the Rust library. The `default_wing_sail_two_element` and `suction_sail_turbosail_charrier_1985` models now give the foil data for positive internal states only, and use the mirroring in the Rust library for negative internal states. The two-element model gives the same result as before. The `suction_sail_turbosail_charrier_1985` model replaces the earlier `default_suction_sail_turbosail` model.
- New `ForceCorrection`, `AddedMass` and `GyroscopicMoment` classes, used in the new `force_correction`, `added_mass` and `gyroscopic_moment` fields in the `WingBuilder`. The `WingBuilder` checks that the gyroscopic moment is only used together with the `RotatingCylinder` section model.

### Breaking changes
- The `cdi_correction_factor`, `added_mass_factor` and `moment_of_inertia_2d` fields were removed from the `Foil` and `RotatingCylinder` classes. Use the new `force_correction`, `added_mass` and `gyroscopic_moment` fields in the `WingBuilder` instead.
- The `write_wake_data_to_file` and `wake_files_folder_path` fields were removed from the `DynamicWakeBuilder`. Use `wake_as_vtk` in pystormbird to export the wake from the script instead.
- The default value of `first_wake_points_direction` in the `DynamicWakeBuilder` is now `Freestream` (was `Chord`), as in the Rust library. Simulations that relied on the old default must set it to `Chord` explicitly.

### Bug fixes
- The input power model and the `RotatingCylinder` model did not match the Rust library, and could produce input that the Rust library would not accept.
- Setting up a rotor sail with the simplified setup failed, due to an unsupported argument to the controller set points.
- Setting up a suction sail with the simplified setup failed, as it used the `default_suction_sail_turbosail` model, which no longer exists. It now uses `suction_sail_turbosail_charrier_1985`.
- Fields that are not set (`None`) are no longer written as `null` in the input for lifting line simulations and complete sail models. Stormbird 0.11.0 does not accept `null` for the new `force_correction`, `added_mass` and `gyroscopic_moment` fields in the `WingBuilder`, so all lifting line simulations set up with stormbird_setup failed. The fields are now left out instead, which gives the default values in Stormbird.

### Other changes
- Added the missing type annotations, so that the package passes the mypy checks set up in `pyproject.toml`. SciPy, which does not include type information, is excluded from the checks.
- The documentation of the `lifting_line_correction` field in the `ActuatorLineBuilder` states that the correction is used by default, and is turned off by setting the field to `None`.

## 0.10.0 - 2026-10-11
Changes up to and including this version were mainly made to keep stormbird_setup in sync with the input format of the Stormbird Rust library. Version 0.9.0 of Stormbird also simplified the imports from stormbird_setup. See the [Stormbird change log](../../stormbird/CHANGELOG.md) for details.
