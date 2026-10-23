# Change log stormbird_setup

This file contains an overview of the changes made to stormbird_setup, the Python library for setting up Stormbird models. See the [Stormbird change log](../../stormbird/CHANGELOG.md) for changes to the underlying models.

## Unreleased
### Highlights
- New `internal_compartment_boundaries` field in the `WingBuilder`, with the same checks as in the Rust library.
- New `SectionModel.suction_sail_turbosail_charrier_1985` model, which is a suction sail model with the `PowerToInternalFlow` conversion, tuned to the TurboSail wind tunnel data from Charrier et al. (1985). The values are generated with the suction sail example in pystormbird.
- Updated to the new data structures for the section models, input power models and controller set points, including the fan model with fan curves. The fan curve data is checked in the same way as in the Rust library. The input power model now matches the variants in the Rust library, and `new_polynomial_rotor_sail_model` gives the power directly, without the `area` argument. The suction sail in the simplified setup now uses the default input power model.
- The `Foil` class was updated with the new and removed fields in the foil model.
- The `RotatingCylinder` class was updated with the new lift parameters. The `rotor_sail_ostman_2023` and `rotor_sail_deybach_2024` models now use parameters fitted to the data in the respective papers.
- New `far_field_ratio` and `initial_relative_wake_length` fields in the `DynamicWakeBuilder`, with the same defaults as in the Rust library.
- The `VaryingFoil` class has the same checks as in the Rust library. The `default_wing_sail_two_element` and `default_suction_sail_turbosail` models now give the foil data for positive internal states only, and use the mirroring in the Rust library for negative internal states. The two-element model gives the same result as before. The TurboSail model gives the same result as before, except for small negative power coefficients, between -0.1187 and 0, where the foils are now the exact mirror image of the foils for positive power coefficients.
- New `ForceCorrection`, `AddedMass` and `GyroscopicMoment` classes, used in the new `force_correction`, `added_mass` and `gyroscopic_moment` fields in the `WingBuilder`. The `WingBuilder` checks that the gyroscopic moment is only used together with the `RotatingCylinder` section model.

### Breaking changes
- The `cdi_correction_factor`, `added_mass_factor` and `moment_of_inertia_2d` fields were removed from the `Foil` and `RotatingCylinder` classes. Use the new `force_correction`, `added_mass` and `gyroscopic_moment` fields in the `WingBuilder` instead.
- The `write_wake_data_to_file` and `wake_files_folder_path` fields were removed from the `DynamicWakeBuilder`. Use `wake_as_vtk` in pystormbird to export the wake from the script instead.
- The default value of `first_wake_points_direction` in the `DynamicWakeBuilder` is now `Freestream` (was `Chord`), as in the Rust library. Simulations that relied on the old default must set it to `Chord` explicitly.

### Bug fixes
- The input power model and the `RotatingCylinder` model did not match the Rust library, and could produce input that the Rust library would not accept.
- Setting up a rotor sail with the simplified setup failed, due to an unsupported argument to the controller set points.

## 0.10.0 - 2026-10-11
Changes up to and including this version were mainly made to keep stormbird_setup in sync with the input format of the Stormbird Rust library. Version 0.9.0 of Stormbird also simplified the imports from stormbird_setup. See the [Stormbird change log](../../stormbird/CHANGELOG.md) for details.
