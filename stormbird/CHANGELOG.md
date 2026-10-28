# Change log Stormbird

This file contains an overview of the changes made to the Stormbird Rust library. The other crates and packages in the repository have their own change logs.

## 0.12.0 - unreleased
### Breaking changes
- The `EmpiricalCirculationCorrection` for actuator line simulations is renamed to `TipLossCorrection`, and the field in the `ActuatorLineBuilder` is renamed from `empirical_circulation_correction` to `tip_loss_correction`. The source file is renamed from `corrections/empirical_circulation.rs` to `corrections/tip_loss.rs`.
- The `overall_correction` factor in the correction is removed. The correction now rejects unknown fields, so input that still contains `overall_correction` gives an error.
- `LiftingLineCorrection::velocity_correction`, and the two functions `velocity_correction_isolated_wings` and `velocity_correction_all_wings`, take two circulation distributions instead of one: the circulation used for the forces on the wings, which is used in the reference model with the small viscous core, and the circulation used for the projected forces, which is used in the model with the viscous core equal to the projection width. Pass the same values for both to get the same behavior as before.
- `LineForceModel::lift_coefficients_pre_stall_with_stall_drop_off` and `LineForceModel::lift_coefficients_post_stall_with_stall_weight` take the velocity at the control points as a new, third, argument, as it is needed by the dynamic stall model.
- The unused `LineForceModel::lift_coefficients_total` function is removed, as it did not include the dynamic stall model.
- `SimulationBuilder::build` panics, and `Simulation::new_from_string` and `CompleteSailModelBuilder::build` return an error, if a wing has a dynamic stall model and the simulation settings are quasi-steady.

### Features
- New, experimental, dynamic stall model for section models that build on the foil model, in the new `section_models::dynamic_stall` module. The model is based on the Leishman-Beddoes model, but adapted to the stall model in the foil model: the pressure lag is applied to the angle of attack, and the separation lag to the amount of stall, which replaces the Kirchhoff relationship. This makes the model valid for all angles of attack, and it always gives the static lift in steady conditions. It is set for each wing with the new, optional, `dynamic_stall` field in the `WingBuilder`, which takes a `DynamicStallModelBuilder` with the two time constants `pressure_time_constant` (default 1.7) and `separation_time_constant` (default 3.0). The model also includes the extra lift from a leading edge vortex, which is fed by the lift lost due to the dynamic stall, created when the lagged angle of attack exceeds the mean stall angle of the foil, and shed after a given non-dimensional time. This is controlled by two more parameters in the builder, `vortex_time_constant` (default 6.0) and `vortex_passage_time` (default 7.0), where a vortex time constant of zero turns the vortex off. The vortex lift is treated in the same way as the post-stall lift. The model is only used in dynamic lifting line simulations, and not yet in actuator line simulations. The drag is not affected. See the dynamic stall chapter in the book for more.
- New `LineForceModel` functions for the dynamic stall model: `dynamic_stall_state`, `has_dynamic_stall`, `set_dynamic_stall_time_step`, `reset_dynamic_stall` and `update_dynamic_stall_state`. The lifting line `Simulation` calls these in `do_step`, and the steady state functions of the `CompleteSailModel` reset the state before each iteration, so that the results are based on the static foil model.
- New `SimulationBuilder::check_settings` function, which checks that the simulation settings are consistent.
- New, experimental, `application` setting in the `TipLossCorrection`, which takes the new `TipLossApplication` enum. The default, `Circulation`, applies the correction to the estimated circulation, as before. With `ProjectedForces`, the correction is only applied to the circulation used for the forces that are projected to the CFD grid, while the forces on the wings are computed from the uncorrected circulation. The correction is then applied after the damping of the circulation. The lifting line correction uses the projected circulation in the model with the viscous core, and the uncorrected circulation in the reference model. The intended use is to stabilize actuator line simulations of rotor sails by smearing the tip vortices in the CFD domain, while keeping the lift-induced drag from the uncorrected circulation. The circulation used for the projected forces is stored in the new `projected_circulation_strength` field of the `ActuatorLine`.
- New `maintain_mean_value` setting in the `TipLossCorrection`, turned on by default. When it is on, the corrected circulation on each wing is scaled so that the mean value over the span, weighted with the length of each line segment, is the same as before the correction. The correction then redistributes the circulation from the ends of the wings towards the middle, rather than reducing it. If the mean value of the corrected circulation is close to zero, the scaling is skipped. Set `maintain_mean_value` to false to get the same behavior as before.

### Changes
- The logic for the tip loss correction is moved from the `ActuatorLine` structure to a new `apply` function on the `TipLossCorrection`.

## 0.11.1 - 2026-10-25
### Features
- Two new, experimental, settings in the `LiftingLineCorrectionBuilder` for actuator line simulations, both turned off by default:
    - `max_correction_ratio`: limits the magnitude of the velocity correction at each control point to a ratio of the magnitude of the sampled velocity. Without a limit, the correction can become larger than the freestream velocity at the ends of wings with a large circulation at the ends, such as rotor sails, which reduces the lift and can make the simulations unstable.
    - `virtual_extension_factors`: virtual extensions of each wing at the first and the last end, only included in the reference model with the small viscous core. The extensions carry the circulation of the outermost line element, and move the tip vortices away from the ends of the wing, which can be used to represent end plates.

### Changes
- The default `induced_velocity_correction_method` in the `Linearized` solver is now `NoCorrection` (was `FullCorrection`). With `FullCorrection`, the circulation is corrected for stall with the angles of attack from the linear solution, while the forces, including the post-stall lift, are calculated with the angles of attack from the corrected circulation. Right before stall, sections could then get both an almost unstalled circulation and post-stall lift, which showed up as a jump in the lift. With `NoCorrection`, the correction and the forces use the same angles of attack, and the lift is smooth through the stall. Models with stall that are tuned with the linearized solver may need to be retuned, or set `FullCorrection` explicitly to keep the old behavior. The initial solution in the `Iterative` solver, with `start_with_linearized_solution`, still uses `FullCorrection`, so that the results of the iterative solver are unchanged.

### Known issues
- The controller and the wind environment use different definitions of the apparent wind direction. The controller measures it for each wing around the span direction, while the wind environment, and the lookup of inflow corrections, measures it around the `wind_rotation_axis`. For sails built from the bottom to the top, with the default wind environment, the two have opposite signs. Set points that are mirrored between positive and negative apparent wind directions are not affected. A fix is targeted for the next release, which will require set points that are not mirrored to be updated.

## 0.11.0 - 2026-10-23
### Highlights
- New structure for the `ActiveAirFlowFoil` section model, intended for suction sails. It now consists of a `VaryingFoil` model for the lift and drag, together with an `InternalStateConversion` that defines how the wing internal state is converted to the internal state of the foil model. Four conversions are available:
    - `SuctionCoefficientDirect`: the wing internal state is the suction coefficient, used directly. Mostly useful for testing.
    - `PowerToPowerCoefficient`: the wing internal state is the power, converted to a power coefficient on each line element. This is the same behavior as for the `ActiveAirFlowFoil` in earlier versions.
    - `PowerToInternalFlow`: the wing internal state is the fan input power, converted to a suction coefficient on each line element using a new, simplified model of the internal flow in the sail, together with a constant fan efficiency. The internal flow model is based on a pressure balance over the porous surface, where the external pressure is a linear function of the suction coefficient, so that the suction coefficient on each line element is found directly, without iterations.
    - `FanSpeedToInternalFlow`: the wing internal state is the rotational speed of the fans, converted to a suction coefficient on each line element using the same internal flow model, together with a fan model based on fan curves. The fan curves can be given either as dimensional curves at a reference rotational speed and density, or as non-dimensional coefficients, and are scaled with the fan affinity laws. The fan model also supports multiple fans in parallel and pressure losses in the ducts between the sail and the fans.
- All section models can now compute a power coefficient. Foils return zero, the `ActiveAirFlowFoil` returns a value consistent with the power given as the wing internal state, and the `RotatingCylinder` uses a polynomial model, with default values from Tillig and Ringsberg (2020).
- Section models can also compute the power for a whole wing, through a new `wing_power` function. For most section models, this is the power coefficient integrated over the wing, while the `ActiveAirFlowFoil` with fan curves computes the power from the operating point of the fans.
- New default input power model, `FromSectionModelPowerCoefficient`, which computes the power for each wing from the section model.
- The controller can now take the internal state set points either in the units of the section model (e.g., spin ratio or suction coefficient), or directly as the wing internal state (e.g., power), through the new `internal_state_set_point_type` setting.
- The internal state of a wing can now vary along the span, through the new `WingInternalState` enum, which is either `Constant` or `SpanVarying`. The span variation is given as non-dimensional span locations and values, and is either linearly interpolated or piecewise constant. It does not need to match the resolution of the line elements. The old way of setting the internal state with one value per wing is kept, and is still what the controller uses. New functions are added for setting span varying internal states. For the power based conversions of the `ActiveAirFlowFoil`, a span varying internal state is interpreted as power per span length.
- Wings can be divided into sealed internal compartments along the span, through the new `internal_compartment_boundaries` field in the `WingBuilder`, for instance to model suction sails with several fans. Each compartment has its own internal pressure and fans, and is solved separately. Compartments are only allowed for section models with an internal flow model.
- Simplified tuning of the `Foil` model, which is also used by the `VaryingFoil` and `ActiveAirFlowFoil` models:
    - The minimum pre-stall drag is now always located at the zero-lift angle, found from the linear part of the lift. The second order drag term is computed from the linear lift divided by the initial lift slope, which is approximately the angle of attack relative to the zero-lift angle. This means that the location of the minimum drag automatically follows changes in camber, flap angle or suction, and no longer needs to be tuned separately.
    - The higher order term in the pre-stall lift was replaced by a reduction of the lift slope, to model separation that develops gradually before the full stall, for instance on a flap. The reduction is set with a factor between 0.0 (no reduction) and 1.0 (zero slope), an onset angle, and a transition range. The factor and onset angle can be set separately for positive and negative angles of attack. The transition is the integral of a sigmoid function, which gives a smooth lift curve, also at zero angle of attack.
- The foil model chapter in the book was extended with equations for the pre-stall lift, the pre-stall drag and the stall model, and the static example figure was replaced by an interactive plot where the parameters of the foil model can be adjusted with sliders.
- The `RotatingCylinder` section model is now a fully parametric model. The data based lift was replaced by a model with an initial lift slope, a different slope at high spin ratios, and a smooth transition between them, set by a transition spin ratio and a transition range. The transition is the integral of a sigmoid function, in the same way as the slope reduction in the `Foil` model, and the lift is an odd function of the spin ratio. The default parameters are fitted to the same CFD data as the old default lift data. The rotating cylinder chapter in the book was rewritten to describe the new model.
- The `VaryingFoil` model can now use one-sided data. If the internal state data only contains values with one sign, the foil is assumed to be symmetric with respect to the sign of the internal state, and an internal state with the opposite sign gives the mirrored foil at the absolute value of the internal state. This means that, for instance, data for a flap angle only has to be given for one side, and can consist of a single flap angle. Data with both positive and negative values is used directly, as before. The interpolation of the foil parameters is also faster, as the two neighboring foils are blended directly, rather than interpolating each parameter separately.
- The quasi-steady wake now keeps the horseshoe vortices and their strengths from the last time step, in a new `QuasiSteadyWake` structure. This means that the induced velocities at arbitrary points, and the export of the wake geometry as VTK data, are now available for both quasi-steady and dynamic wakes, through the `induced_velocities` and `wake_as_vtk` functions on the simulation. A quasi-steady wake is exported with one polygon per horseshoe vortex. The VTK export for both wake types uses a shared writer, and the output for the dynamic wake is unchanged.
- New wing level force models, which are set for each wing in the `WingBuilder`, independent of the section model, and are all turned off by default:
    - `force_correction`: a `ForceCorrection` structure for empirical corrections of the forces. For now, it contains the `lift_induced_drag_correction`, which adds a drag coefficient equal to the factor times the square of the lift coefficient. The lift coefficient is computed from the total lift force on each line element, i.e., the circulatory force and the viscous lift.
    - `added_mass`: an `AddedMass` strip theory model, with separate coefficients for the acceleration normal to and along the chord. The coefficients are relative to the added mass of a flat plate, so that the theoretical value is 1.0 for acceleration normal to a foil, and 1.0 in all directions for a circular cylinder.
    - `gyroscopic_moment`: a `GyroscopicMoment` model for rotor sails, set by the total moment of inertia of the rotor. Only allowed together with the `RotatingCylinder` section model.
- The force calculations chapter in the book has new sections on the force corrections, the added mass model and the gyroscopic moment model, including recommended values for the added mass coefficients, and a comparison between the lift-induced drag correction and virtual line segments.
- The controller can again apply a moving average to its input, through the `moving_average_window_size` setting in the `ControllerBuilder`. When the window size is larger than zero, the measured angle of attack, velocity and apparent wind direction for each wing are averaged over the given number of controller updates, so that the controller acts on a smoothed version of the flow conditions rather than the instantaneous values. The apparent wind direction is averaged through its sine and cosine, so that the average is correct also when the direction passes ±π. The filter is not used by default.

### Breaking changes
- The input format for the `ActiveAirFlowFoil` section model has changed. The `VaryingFoil` data must now be given in a `varying_foil` field, together with an `internal_state_conversion` field. To get the same behavior as before, use the `PowerToPowerCoefficient` conversion.
- The default input power model has changed from `NoPower` to `FromSectionModelPowerCoefficient`. This gives the same result for wing sails, but rotor sails without an explicit input power model will now get the power from the polynomial power model in the `RotatingCylinder`. Set the input power model to `NoPower` explicitly to get the old behavior.
- Fields in the controller set points were renamed to reflect that the set points are no longer always given in the units of the section model, and that the limits are applied to the wing internal state:
    - `section_model_internal_state_data` -> `internal_state_data`
    - `internal_state_max_value` -> `wing_internal_state_max_value`
    - `max_internal_state_change_rate` -> `wing_internal_state_max_change_rate`
- The `input_power` function on the line force model now takes the sectional force input as an argument, as the power can depend on the local flow conditions.
- Stormbird now depends on version 0.4.0 of Stormath, which adds a bracketed root-finding method (Brent's method), used to find the operating point of the fans. Stormath 0.4.0 requires Rust 1.85, which is therefore also the minimum supported Rust version for Stormbird, set through the new `rust-version` field.
- The `internal_state` field in the line force model is now a vector of `WingInternalState` rather than a vector of floats. Use `set_internal_state` to set constant values as before, and `internal_state_representative_values` to get one value per wing.
- Virtual line elements are no longer included in the power or in the flow through an `ActiveAirFlowFoil`. They still get the same type of internal state as the rest of the wing, so that the circulation stays consistent along the span. For the `PowerToPowerCoefficient` conversion, this means that the power is only distributed over the line elements that are not virtual.
- Changes to the fields of the `Foil` model. As unknown fields are not accepted, input that uses the removed fields must be updated:
    - `angle_cd_min` was removed, as the location of the minimum drag is now computed from the linear lift. This field also shifted the stall angles, which are now always measured from zero angle of attack. Use `mean_positive_stall_angle` and `mean_negative_stall_angle` to adjust the stall angles instead.
    - `cl_high_order_factor_positive`, `cl_high_order_factor_negative` and `cl_high_order_power` were removed, and replaced by `cl_slope_reduction_factor_positive`, `cl_slope_reduction_factor_negative`, `cl_slope_reduction_onset_angle_positive`, `cl_slope_reduction_onset_angle_negative` and `cl_slope_reduction_range`. The slope reduction is off by default.
    - For foils with camber, flap or suction, and a non-zero `cd_second_order_factor`, the pre-stall drag will be different from before, as the drag is now symmetric around the zero-lift angle.
    - `cd_bump_during_stall` and `cd_stall_angle_offset` were removed, as they were experimental and rarely used. The drag now always uses the same amount of stall as the lift, and there is no additional drag term during the stall transition.
- The `cdi_correction_factor` and `added_mass_factor` fields were removed from the `Foil` and `RotatingCylinder` section models, and the `moment_of_inertia_2d` field was removed from the `RotatingCylinder`, as they are properties of the whole wing rather than of the two-dimensional section. As unknown fields are not accepted, input that uses these fields must be updated:
    - `cdi_correction_factor` is replaced by `lift_induced_drag_correction` in the new `force_correction` field in the `WingBuilder`. The lift coefficient in the correction is now computed from the total lift force on each line element, rather than from the section model, which gives slightly different results, mainly after stall. The correction is also no longer included in the drag coefficient from the section models, e.g., when plotting the section model in Python.
    - `added_mass_factor` is replaced by the new `added_mass` field in the `WingBuilder`. The definition has changed: the old factor was multiplied with the density, the area of the line element and the acceleration, which means that it implicitly had the unit of length. The new coefficients are non-dimensional, and relative to the added mass of a flat plate. For foils, the old factor corresponds to a `normal_coefficient` of `4 * added_mass_factor / (pi * chord_length)`.
    - `moment_of_inertia_2d` is replaced by `moment_of_inertia` in the new `gyroscopic_moment` field in the `WingBuilder`, which is the total moment of inertia of the rotor, rather than the moment of inertia per span length.
- The gyroscopic effect is now computed as a pure moment. Previously, it was computed with the wrong units, stored as a sectional force, included in the total force, and used as a force when computing the moments. The `gyroscopic` field in the `SectionalForces` is renamed to `gyroscopic_moment`, is not included in the `total` force, and is added directly to the integrated moments. The `gyroscopic` field in the `IntegratedValues` is therefore always zero for the integrated forces.
- The `sectional_added_mass_force` function on the line force model no longer takes the internal states as an argument, and `sectional_gyroscopic_force` is replaced by `sectional_gyroscopic_moments`. The `added_mass_factor` function on the `SectionModel`, and the `added_mass_coefficient` functions on the section models, were removed.
- The `spin_ratio_data` and `cl_data` fields in the `RotatingCylinder` model were removed, and replaced by `cl_initial_slope`, `cl_high_spin_ratio_slope`, `cl_slope_transition_spin_ratio` and `cl_slope_transition_range`. As unknown fields are not accepted, input that uses the removed fields must be updated. The lift with the default parameters deviates up to about 0.4 in lift coefficient from the old default data.
- The default values of the `DynamicWakeBuilder` were synchronized with stormbird_setup. `last_panel_relative_length` is now 1.0 (was 0.25), as in stormbird_setup. Simulations set up directly with JSON, or through other interfaces than stormbird_setup, that relied on the old default must set this value explicitly. The `last_panel_value` in the `SinIncrease` viscous core length evolution now has a default value, `Relative(0.1)`, as in stormbird_setup.
- `VaryingFoil` models with only positive (or only negative) internal state data now give the mirrored foil for internal states with the opposite sign. Previously, the first (or last) foil in the data was used for these internal states. The input is also checked when it is read: the internal state data must be strictly increasing, contain at least one value, and have the same length as the foils data.
- The automatic export of the dynamic wake to files was removed, together with the `write_wake_data_to_file` and `wake_files_folder_path` settings in the `DynamicWakeBuilder`. As unknown fields are not accepted, input that uses these settings must be updated. The wake can instead be exported from the script that runs the simulation, with the `wake_as_vtk` function. `Simulation::dynamic_wake_as_vtk` is deprecated in favor of `Simulation::wake_as_vtk`, which returns the data directly rather than an `Option`, as all wakes can now be exported. `WakeData::QuasiSteady` now holds a `QuasiSteadyWake` rather than the `QuasiSteadyWakeSettings`.

- The `check_for_zero_linear_lift` function on the `Linearized` solver was removed. It was meant to skip the solver when there is no lift, but due to a bug it never did, so the results are unchanged.
- `EllipticalWing::lift_induced_angle_of_attach` was renamed to `EllipticalWing::lift_induced_angle_of_attack`, to fix a typo.

### Bug fixes
- The `moving_average_window_size` setting in the `ControllerBuilder` was accepted, but not used, since the restructuring of the controller in version 0.8.0.
- The initial shape of the dynamic wake, before the first time step, had a length proportional to the chord length squared, rather than the chord length, as the direction of the wake was not normalized. The wake is initialized again at the first time step, so this only affected the wake shape before the simulation started.
- The angular velocity computed from finite differences of the rotation was wrong when a rotation angle passed -π in the negative direction, i.e., jumped from close to -π to close to π. This is fixed in Stormath 0.4.0, and affects simulations where the rotation is set with `set_translation_and_rotation_with_finite_difference_for_the_velocity`.

### Other changes
- Removed the unused `rand` dependency.
- Removed the `empirical_models` folder, which contained an old model for superstructure forces that was no longer part of the library.
- Added `keywords` and `categories` to the crate metadata, and changed the license identifier from the deprecated `GPL-3.0` to `GPL-3.0-only`.
- Updated the description in the README, which is also the front page of the documentation, to include all three simulation methods.
- Fixed the list of methods in the documentation of the force calculations, which included a function that no longer exists and was missing several others.
- Added documentation to all public modules, structures, fields, variants and functions that were missing it.
- Fixed errors in the existing documentation, including argument lists that did not match the function parameters, descriptions that referred to old behavior (e.g., the circulation smoothing being Gaussian only, and the wing builder interpolating the section model), and a number of typos.
- Internal cleanup based on Clippy suggestions, without changes to the behavior. The `to_string` functions on the section models were replaced by implementations of the standard `Display` trait, which give the same JSON string through `to_string`.
- Fixed tests that failed to compile with the `padded_spatial_vectors` feature, and tests that failed or gave warnings with the `single_precision` feature. The tests for the sign of the forces and moments from motion now compare against the velocity used in the simulation, rather than the analytical velocity, which could have the wrong sign close to the turning points of the motion.

## 0.10.0 - 2026-10-11
### Highlights
- New steady state mode for the dynamic wake, intended to find a steady solution faster when a very dynamic wake shape is expected
- Small updates to the controller settings to simplify it

## 0.9.0 - 2026-06-09
### Highlights
- New wind models, including different atmospheric boundary layer models and gust spectrums
- Added the option to use "virtual line elements" to simulate end-plates and other similar mechanisms for reducing lift-induced velocities

### Bug fixes
- Fixed bug in the effective angle of attack controller, introduced in version 0.8.0
- Added proper sign for the residual calculation
- Removed spanwise velocities in the linearized solver
- Fixed issue in the actuator line when `realign_sectional_forces` were set to true. Previously this only worked when the lift was positive. Now it handles negative lift as well.

## 0.8.0 - 2026-02-25
### Highlights
- Refactoring of control system to handle different setup when using multiple sails
- Fix of bug and refactoring of the input power model
- Small updates to the foil model

## 0.7.0 - 2025-10-30
### Highlights
- Cleanup and restructuring of the actuator line functionality. In particular with focus on finalizing the correction methods, and removing methods that were tested but not found useful.

## 0.6.0 - 2025-10-21
### Highlights
Internal rewrite of some functionality, with the goal of simplifying and speeding up the code. Some new features were also added.

- Now storing global versions of geometry data directly in the line force model. This was to, potentially, avoid unnecessary repeated transformations of the local geometry when evaluating the forces and updating the wake
- New linearized solver, as a faster alternative to the full non-linear solver.
- New ways (or rather, back to an old way) to create a steady wake; direct from horseshoe vortices, rather through a reduced dynamic wake structure as before. A little faster.
- Start of "complete sail model" functionality, where the goal is to collect the typical functionality needed to model a "complete" sail for route simulation. This includes a lifting line model for the sails, controllers and a model of the wind conditions.
- Added empirical models for input power to the line force model

## 0.5.2 - 2025-02-06
### Highlights
No significant changes to the core library. This update was mostly about small changes to the FMU version, to better facilitate hybrid testing of wind-powered ships. The background was the first test of Stormbird in a hybrid test campaign in the SeaZero project.

In particular, the interface to the FMU was changed to fit better with the needs in such experiments. This included the following:
- Possibility of applying Froude model scaling to the input (from model scale to full-scale) and output (from full-scale to model scale)
- All parameters were moved to separate JSON files. The reason was simply that the current lab software used for doing hybrid tests only supports float variables directly in the FMU. There was therefore a need to move all non-float parameters to a separate interface. To make the setup consistent, all parameters were therefore moved to a separate JSON file. The file path can still be specified in the FMU, but it also has a default value suitable for hybrid testing (the default value only makes sense for the hybrid testing setup, so please specify the path in any other case)
- Adding the possibility of specifying "translational velocity" as input to the FMU. This is an alternative to applying motion directly through updates to the position, and instead specify the motion velocity directly. That is, the velocity due to translational motion, not rotational (that might come later but was not necessary in this case)

## 0.5.1 - 2024-10-24
### Bug fixes
- Fixed a bug in the dynamic wake update. The last and first panels in the dynamic wake were not updated entirely correctly when the wing was moving. The last panel was only moved in the direction of the freestream velocity, if the last panel length was set to be longer than the time step would indicate, the motion became incorrect. The second panel behind the line force model was also updated wrong, as the position was integrated in time from the current version of the first panel, not the previous. This caused a small "bend" in the wake when the wing was moving.

## 0.5.0 - 2024-10-21
### Highlights
- Some internal clean up of methods and structures
- It is now possible to get forces in *either* a global coordinate system or a body-fixed coordinate system. The line force model has an additional field which specifies the coordinate system for the force output. This can be specified in the builder input JSON string. See the documentation for more.
- The FMU version was updated, in preparation for more active use in the future. Expect more updates to the FMU version in future releases.

### Bug fixes
- A bug was discovered related to how `local_wing_angles` were applied to the line force model. It previously rotated the wings in a global coordinate system, which caused errors when the position of the wings were different than zero.

## 0.4.0 - 2024-09-16
### Highlights
Mostly a restructuring of the internal data, with the goal of either simplifying, or preparing for new features in the future. However, some external changes were also necessary (see below).

- The dynamic and quasi-steady wake was merged into one common data structure. This makes it much easier to ensure that all functionality is synced between the two simulation modes
- The lifting line solver setup was rewritten, mostly to make it easier to incorporate new solvers in the future.
- A new interpolation mode was added to the actuator line simulations; it is now possible to use the built in interpolation in OpenFOAM, rather than Gaussian interpolation in Stormbird.
- Gaussian smoothing and prescribed circulation distribution was merged into one Enum - it does not make sense to use both of them at the same time, so now the user have to explicitly choose only one.
- The line force model must be created with hints on whether or not the circulation will be zero at the ends or not. This is useful for initialization and when applying circulation corrections.
- A new `prescribed_initialization` function was added so that the circulation strength can be initialized with a prescribed shape. This can sometimes reduce the number of iterations necessary.
- The Vec3 data structure was replaced with a more general structure called SpatialVector. The main reason is that this data structure is written to also be useful in other rust libraries, and it is practical with shared functionality
- Moved the velocity corrections from the wake model to the solver, to simplify the wake data structure.
