# Change log stormbird_setup

This file contains an overview of the changes made to stormbird_setup, the Python library for setting up Stormbird models. See the [Stormbird change log](../../stormbird/CHANGELOG.md) for changes to the underlying models.

## 0.12.0 - 2026-10-31
### Breaking changes
- `InflowCorrectionsSingleDirection.from_simulation_results_aligned_with_freestream` is replaced by `InflowCorrectionsSingleDirection.from_simulation_result`, which takes the wind environment, the wind condition and the ship velocity as input, rather than a single freestream velocity. The measured velocity at each control point is compared with the undisturbed apparent wind at the same location, so the function also works with inflow conditions that vary with height. The angle correction is computed from the velocity vectors, around the up direction, rather than from the angle of attack, so the sails no longer need to be aligned with the flow.
- Requires Stormbird 0.12.0 or newer for actuator line simulations.
- The `EmpiricalCirculationCorrection` is renamed to `TipLossCorrection`, and the field in the `ActuatorLineBuilder` is renamed from `empirical_circulation_correction` to `tip_loss_correction`, following the changes in Stormbird 0.12.0.
- The `overall_correction` field in the correction is removed, and replaced by the new `maintain_mean_value` field, which is true by default. When it is true, the corrected circulation on each wing is scaled so that the mean value over the span is the same as before the correction. Set it to false to get the same behavior as before.
- The `slip_wall_boundary_override` field of the Stormflow `SimulationBuilder` is removed, and replaced by the new `boundary_condition_type` field, which takes the new `BoundaryConditionType` enum: `WindInletOutlet` (default, as before) or `WindTunnel`, where the x axis goes from the inlet to the outlet, and all the faces along the y and z axes are slip walls. Requires Stormflow 0.3.0 or newer.

### Other changes
- The internal flow model of the `SuctionSailCharrier1985` default sail is retuned, with `cp_no_suction = -0.5`, `cp_suction_gradient = -74.945` and `loss_coefficient = 21.414`. The tuning in the pystormbird example is now bounded, so that the parameters are valid, and the new values are reproduced by the example. The power curve matches the experiments about equally well as before.
- New force tuning of the inflow corrections in the wind environment, with the new `InflowCorrectionsForceTuner` class in `stormbird_setup.wind`, together with `ForceTuningCase`, `ForceTuningSettings`, `ForceTuningCaseResult` and `ForceTuningResult`. The tuner finds one magnitude correction and one angle correction for each sail, so that the forces from a quasi-steady lifting line simulation match measured forces, for instance from CFD simulations with actuator lines or the full geometry of the sails. It requires pystormbird 0.12.0 or newer, which is a new, optional, dependency that can be installed with `pip install stormbird_setup[force-tuning]`. The rest of stormbird_setup does not depend on pystormbird.
- New `WindEnvironment.linear_velocity_from_ship_velocity` and `WindEnvironment.apparent_wind_direction_from_condition_and_linear_velocity` functions, which use the same definitions as Stormbird.
- New, experimental, `DynamicStallModelBuilder` class in `stormbird_setup.section_models`, and a new, optional, `dynamic_stall` field in the `WingBuilder` and the `SailDefinition`, following the new dynamic stall model in Stormbird 0.12.0. The builder has four optional time constants for the stall: `pressure_time_constant`, `separation_time_constant`, and, for the leading edge vortex, `vortex_time_constant` and `vortex_passage_time`, which can only be set for section models that build on the foil model. In addition, it has the experimental `internal_state_time_constant`, which can be used for all section models, and is mostly intended for rotor sails. The model can only be used together with dynamic simulation settings. Both checks are done by the `WingBuilder` and the lifting line `SimulationBuilder`. New `DynamicStallModelBuilder.has_foil_parameters` function. The field is only included in the JSON output when it is set. The new `LineForceModelBuilder.has_dynamic_stall` function returns true if any of the wings have a dynamic stall model.
- The dynamic stall model can also be used in actuator line simulations, following the changes in Stormbird 0.12.0. The `ActuatorLineBuilder` gives a warning if a wing has a dynamic stall model and the `damping_factor` in the `SolverSettings` is below one, as the damping acts as an additional time lag on the circulation.
- New `application` field in the `TipLossCorrection`, which takes the new `TipLossApplication` enum (`Circulation` by default, or `ProjectedForces`), following the changes in Stormbird 0.12.0. With `ProjectedForces`, the correction is only applied to the forces that are projected to the CFD grid. The enum is exported from `stormbird_setup.actuator_line`.
- The OpenFOAM `SimulationBuilder` has the same new `boundary_condition_type` field, using the same `BoundaryConditionType` enum, which is also exported from `stormbird_setup.openfoam`. With `WindTunnel`, the `vertical_sides`, `z_min` and `z_max` patches of the base folder are replaced by the patches `inlet`, `outlet` and `tunnel_walls`, which reuse the conditions of the vertical sides, the top and the bottom, respectively. The apparent wind must be along the positive x axis, which is checked when the builder is created. The default, `WindInletOutlet`, gives the same case as before.

## 0.11.3 - 2026-10-24
### highlights
- New `default_sails` module with default sail models, classified by the `DefaultSail` enum. Each model is either a best guess for a generic sail, or tuned to a specific source in the literature: `GenericSingleElementWingSail`, `GenericTwoElementWingSail`, `RotorSailDeybach2024`, `RotorSailOstman2023` and `SuctionSailCharrier1985`.
- Each default sail gives a `SailDefinition`, which contains the section model, controller set points, solver settings and circulation correction, as well as optional virtual extensions, force corrections and added mass. Models that are tuned to data include the simulation settings from the tuning. For instance, `SuctionSailCharrier1985` uses Gaussian smoothing and a virtual extension at the top, as these settings affect the lift at high suction. The definition is independent of the sail size, and can be scaled to different chord lengths and heights. The maximum revolutions per second of the rotor sails is given as a maximum tip speed, so that it scales with the diameter. The definition is a normal Pydantic model, so custom definitions can be stored as JSON files.
- Each default sail is available for both the linearized and the iterative lifting line solver, selected with the `SolverType` enum, where the linearized solver is the default. The only difference is the stall angles and, for the suction sail, the stall ranges of the foils, which are set so that the maximum lift, and the angle of attack where it occurs, are about the same for both solvers.
- New `CompleteSailModelBuilder.new_from_default_sail` class method, which sets up a complete sail model from a default sail or a custom definition, the sail positions, and the chord length and height. The sails are oriented after the up direction and zero direction vector in the wind environment. Sails with the base on the symmetry plane get a non-zero circulation at the base, which is used by the circulation corrections, while sails above the plane, e.g., on a deck, get zero circulation at both ends.
- Disks are now supported as geometries in OpenFOAM cases, for instance as end disks on rotor sails. They are meshed as solid cylinders, so they must have a thickness larger than zero, and the fillet radius must be zero.
- The `LiftingLineCorrectionBuilder` has the new, experimental, settings `max_correction_ratio` and `virtual_extension_factors`. They require a version of Stormbird newer than 0.11.0, and are only included in the JSON output when they are set, so the output is unchanged for older versions.

### Bug fixes
- OpenFOAM cases failed when stormbird_setup was built on Windows, as the files in the base folder got CRLF line endings, which bash and OpenFOAM do not accept. `write_foam_case` now converts all text files in the case folder to LF line endings, and the repository enforces LF line endings with a `.gitattributes` file.
- The refinement around the actuator lines in OpenFOAM cases was skipped when the refinement zone was small relative to the background cells, as snappyHexMesh only refines the cells with their center inside the zone. The refinement cylinder is now extended by half the diagonal of a background cell, both in radius and at the ends, so that all cells that overlap the zone are refined.
- The controller set points for the suction sail were given as a power coefficient of 0.3, while the section model uses the suction coefficient as the internal state. The set points are now given as suction coefficients, with a maximum of 0.05, which is the highest suction coefficient in the tuned data.

### Breaking changes
- The default models were moved from the `SectionModel` and `ControllerSetPoints` classes to the `default_sails` module. The `SectionModel` class methods `rotor_sail_deybach_2024`, `rotor_sail_ostman_2023`, `default_wing_sail_single_element`, `default_wing_sail_two_element` and `suction_sail_turbosail_charrier_1985`, and the `ControllerSetPoints` class methods `new_default_wing_sail_single_element`, `new_default_wing_sail_two_element`, `new_default_rotor_sail` and `new_default_suction_sail` are removed. Use, e.g., `DefaultSail.RotorSailDeybach2024.section_model()` and `DefaultSail.RotorSailDeybach2024.controller_set_points(diameter)` instead.
- `SimpleSailSetup` and `SingleWingSimulation` are removed from the `simplified_setup` module. Use the `default_sails` module instead.
- The default rotor sails now use the input power from the section model, instead of the polynomial input power model.
- The maximum angle of attack set points of the default wing sails and suction sail are 5 degrees below the angle of attack where the 2D lift of the foils is at its maximum (at the maximum flap angle or suction coefficient), for each solver type. This gives 12.26 degrees for the single element wing sail (was 15), 6.16 degrees for the two-element wing sail (was 12), and 24.78 degrees for the suction sail (was 27), for the linearized solver.
- The angle of attack set points of the default wing sails and suction sail are effective angles of attack (`use_effective_angle_of_attack = True`), so that multiple sails are operated close to the set points, also when they affect each other.
- The `linearized` argument for the Deybach (2024) rotor sail model is removed.
- The default `induced_velocity_correction_method` in the `Linearized` solver is now `NoCorrection` (was `FullCorrection`), as in the Rust library, which avoids a jump in the lift right before stall. The value is always included in the JSON output, so this also changes the results with older versions of Stormbird.

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
