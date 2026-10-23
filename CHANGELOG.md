# Change log Stormbird

This file contains an overview of the changes made to the stormbird library. The file targets both the core library an all interfaces to it in one, as changes to the interfaces usually will be due to changes in the core library.

## 0.4.0 - 2024-09-16

### Highlights
Mostly a restructuring of the internal data, with the goal of either simplifying, or preparing for new features in the future. However, some external changes was also necessary (see below).

### Changes to the Rust library
- The dynamic and quasi-steady wake was merged into one common data structure. This makes it much easier to ensure that all functionality is synced between the two simulation modes
- The lifting line solver setup was rewritten, mostly to make it easier to incorporate new solvers in the future.
- A new interpolation mode was added to the actuator line simulations; it is now possible to use the built in interpolation in OpenFOAM, rather than Gaussian interpolation in Stormbird.
- Gaussian smoothing and prescribed circulation distribution was merged into one Enum - it does not make sense to use both of them at the same time, so now the user have to explicitly choose only one.
- The line force model must be created with hints on whether or not the circulation will be zero at the ends or not. This is useful for initialization and when applying circulation corrections.
- A new `prescribed_initialization` function was added so that the circulation strength can be initialized with a prescribed shape. This can sometimes reduce the number of iterations necessary.
- The Vec3 data structure was replaced with a more general structure called SpatialVector. The main reason is that this data structure is written to also be useful in other rust libraries, and it is practical with shared functionality
- Moved the velocity corrections from the wake model to the solver, to simplify the wake data structure.

### Changes to the Python library
- The changes to the interface on the rust side also affects the Python side, as Python still relies on JSON strings to set up models. See the updated examples for how to set up models. The most important change is perhaps the change in the spatial vector name, from `Vec3` to `SpatialVector`.

## 0.5.0 - 2024-10-21
### Highlights
- Some internal clean up of methods and structures
- It is now possible to get forces in *either* a global coordinate system or a body-fixed coordinate system
- The FMU version was updated, in preparation for more active use in the future. Expect more updates to the FMU version in future releases.

### Bug fixes
- A bug was discovered related to how local_wing_angles where applied to the line force model. It previously rotated the wings in a global coordinate system, which caused errors when the position of the wings where different than zero.

### Changes to the Rust library
- The line force model have gotten an additional field which specifies the coordinate system for the force output. This can be specified in the builder input JSON string. See the documentation for more.

### Changes to the Python library
- No changes directly, as the only change is for the setup of the line force model, which depends on a JSON string.

## 0.5.1 - 2024-10-24
### Bug fixes
- Fixed a bug in the dynamic wake update. The last and fist panels in the dynamic wake was not updated entirly correctly when the wing was moving. The last panel was only moved in the direction of the freestream velocity, if the last panel length was set to be longer than the time step would indicate, the motion became incorrect. The second panel behind the line force model was also updated wrong, as the position was integrated in time from the current version of the first apnel, not the previous. This caused a small "bend" in the wake when the wing was moving.

## 0.5.2 - 2025-02-06
### Highlights
This update was mostly about small changes to the FMU version, to better facilitate hybrid testing of wind-powered ships. The background was the first test of Stormbird in a hybrid test campaign in the SeaZero project.

In particular, the interface to the FMU was changed to fit better with the needs in such experiments. This included the following:
- Possibility of applying Froude model scaling to the input (from model scale to full-scale) and output (from full-scale to model scale)
- All parameters where moved to separate JSON files. The reason was simply that the current lab software used for doing hybrid tests only supports float variables directly in the FMU. There was therefore a need to move all non-float parameters to a separate interface. To make the setup consistent, all parameters where therefore moved to a separate JSON file. The file path can still be specified in the FMU, but it also has a default value suitable for hybrid testing (the default value only makes sense for the hybrid testing setup, so please specify the path in any other case)
- Adding the possibility of specifying "translational velocity" as input to the FMU. This is an alternative to applying motion directly through updates to the position, and instead specify the motion velocity directly. That is, the velocity due to translational motion, not rotational (that might come later but was not necessary in this case)

## 0.6.0 - 2025-10-21
### Highlights
Internal rewrite of some functionality, with the goal of simplifying and speeding up the code. Some new features were also added.

- Now storing global versions of geometry data directly in the line force model. This was to, potentially, avoid unnecessary repeated transformations of the local geometry when evaluating the forces and updating the wake
- New linearized solver, as a faster alternative to the full non-linear solver.
- New ways (or rather, back to an old way) to create a steady wake; direct from horseshoe vortices, rather through a reduced dynamic wake structure as before. A little faster.
- Start of "complete sail model" functionality, where the goal is to collect the typical functionality needed to model a "complete" sail for route simulation. This includes a lifting line model for the sails, controllers and a model of the wind conditions.
- Added empirical models for input power to the line force model

## 0.7.0 - 2025-10-30
### Highlights
- Cleanup and restructuring of the actuator line functionality. In particular with focus on finalizing the correction methods, and removing methods that were tested but not found useful.

## 0.8.0 - 2026-02-25
### Highlights
- Refactoring of control system to handle different setup when using multiple sails
- Fix of bug and refactoring of the input power model
- Small updates to the foil model

## 0.9.0 - 2026-06-09
### Highlights
- New wind models, including different atmospheric boundary layer models and gust spectrums
- Added the option to use "virtual line elements" to simulate end-plates and other similar mechanisms for reducing lift-induced velocities
- Addition of a new crate called *Stormflow*, which is a custom CFD solver specialized for simple actuator line simulations
- Simplified stormbird_setup imports
### Bug fixes
- Fixed bug in the effective angle of attack controller, introduced in version 0.8.0
- Added proper sign for the residual calculation
- Removed spanwise velocities in the linearized solver
- Fixed issue in the actuator line when `realign_sectional_forces` where set to true. Previously this only worked when he lift was positive. Now it handles negative lift as well.

## 0.10.0 - 2026-10-11
### Highlights
- New steady state mode for the dynamic wake, intended to find a steady solution faster when a very dynamic wake shape is expected
- Small updates to the controller settings to simplify it
- Many updates to the Stormflow crate, making it ready for first "official" version (v0.1.0)

## 0.11.0 - Unreleased
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

### Breaking changes
- The input format for the `ActiveAirFlowFoil` section model has changed. The `VaryingFoil` data must now be given in a `varying_foil` field, together with an `internal_state_conversion` field. To get the same behavior as before, use the `PowerToPowerCoefficient` conversion.
- The default input power model has changed from `NoPower` to `FromSectionModelPowerCoefficient`. This gives the same result for wing sails, but rotor sails without an explicit input power model will now get the power from the polynomial power model in the `RotatingCylinder`. Set the input power model to `NoPower` explicitly to get the old behavior.
- Fields in the controller set points were renamed to reflect that the set points are no longer always given in the units of the section model, and that the limits are applied to the wing internal state:
    - `section_model_internal_state_data` -> `internal_state_data`
    - `internal_state_max_value` -> `wing_internal_state_max_value`
    - `max_internal_state_change_rate` -> `wing_internal_state_max_change_rate`
- The `input_power` function on the line force model now takes the sectional force input as an argument, as the power can depend on the local flow conditions.
- Stormbird now depends on version 0.4.0 of Stormath, which adds a bracketed root-finding method (Brent's method), used to find the operating point of the fans.
- The `internal_state` field in the line force model is now a vector of `WingInternalState` rather than a vector of floats. Use `set_internal_state` to set constant values as before, and `internal_state_representative_values` to get one value per wing.
- Virtual line elements are no longer included in the power or in the flow through an `ActiveAirFlowFoil`. They still get the same type of internal state as the rest of the wing, so that the circulation stays consistent along the span. For the `PowerToPowerCoefficient` conversion, this means that the power is only distributed over the line elements that are not virtual.
- Changes to the fields of the `Foil` model. As unknown fields are not accepted, input that uses the removed fields must be updated:
    - `angle_cd_min` was removed, as the location of the minimum drag is now computed from the linear lift. This field also shifted the stall angles, which are now always measured from zero angle of attack. Use `mean_positive_stall_angle` and `mean_negative_stall_angle` to adjust the stall angles instead.
    - `cl_high_order_factor_positive`, `cl_high_order_factor_negative` and `cl_high_order_power` were removed, and replaced by `cl_slope_reduction_factor_positive`, `cl_slope_reduction_factor_negative`, `cl_slope_reduction_onset_angle_positive`, `cl_slope_reduction_onset_angle_negative` and `cl_slope_reduction_range`. The slope reduction is off by default.
    - For foils with camber, flap or suction, and a non-zero `cd_second_order_factor`, the pre-stall drag will be different from before, as the drag is now symmetric around the zero-lift angle.
    - `cd_bump_during_stall` and `cd_stall_angle_offset` were removed, as they were experimental and rarely used. The drag now always uses the same amount of stall as the lift, and there is no additional drag term during the stall transition.
- The `spin_ratio_data` and `cl_data` fields in the `RotatingCylinder` model were removed, and replaced by `cl_initial_slope`, `cl_high_spin_ratio_slope`, `cl_slope_transition_spin_ratio` and `cl_slope_transition_range`. As unknown fields are not accepted, input that uses the removed fields must be updated. The lift with the default parameters deviates up to about 0.4 in lift coefficient from the old default data.
- The default values of the `DynamicWakeBuilder` were synchronized between the Rust library and stormbird_setup. In the Rust library, `last_panel_relative_length` is now 1.0 (was 0.25), as in stormbird_setup. Simulations set up directly with JSON, or through other interfaces than stormbird_setup, that relied on the old default must set this value explicitly. In stormbird_setup, `first_wake_points_direction` is now `Freestream` (was `Chord`), as in the Rust library. Simulations set up with stormbird_setup that relied on the old default must set it to `Chord` explicitly. The `last_panel_value` in the `SinIncrease` viscous core length evolution now has a default value, `Relative(0.1)`, as in stormbird_setup.
- `VaryingFoil` models with only positive (or only negative) internal state data now give the mirrored foil for internal states with the opposite sign. Previously, the first (or last) foil in the data was used for these internal states. The input is also checked when it is read: the internal state data must be strictly increasing, contain at least one value, and have the same length as the foils data.
- The automatic export of the dynamic wake to files was removed, together with the `write_wake_data_to_file` and `wake_files_folder_path` settings in the `DynamicWakeBuilder`. As unknown fields are not accepted, input that uses these settings must be updated. The wake can instead be exported from the script that runs the simulation, with the `wake_as_vtk` function. `Simulation::dynamic_wake_as_vtk` is deprecated in favor of `Simulation::wake_as_vtk`, which returns the data directly rather than an `Option`, as all wakes can now be exported. `WakeData::QuasiSteady` now holds a `QuasiSteadyWake` rather than the `QuasiSteadyWakeSettings`.

### Changes to the Python library
- pystormbird: new `ActiveAirFlowFoil` class, including functions for the section internal states and the power for a whole wing, and a new `power_coefficient` function for the `RotatingCylinder` class. The `as_active_air_flow_foil` function on the `VaryingFoil` class was removed, as an active air flow foil now also needs an internal state conversion.
- pystormbird: new `set_internal_state_linear` and `set_internal_state_piecewise_constant` functions for the lifting line simulation and the complete sail model, for setting internal states that vary along the span. The `internal_state` functions return the mean value over the span for wings with a span varying internal state.
- stormbird_setup: new `internal_compartment_boundaries` field in the `WingBuilder`, with the same checks as in the Rust library.
- stormbird_setup: new `SectionModel.suction_sail_turbosail_charrier_1985` model, which is a suction sail model with the `PowerToInternalFlow` conversion, tuned to the TurboSail wind tunnel data from Charrier et al. (1985). The values are generated with the suction sail example in pystormbird.
- stormbird_setup: updated to the new data structures for the section models, input power models and controller set points, including the fan model with fan curves. The fan curve data is checked in the same way as in the Rust library. The input power model now matches the variants in the Rust library, and `new_polynomial_rotor_sail_model` gives the power directly, without the `area` argument. The suction sail in the simplified setup now uses the default input power model.
- pystormbird and stormbird_setup: the `Foil` class was updated with the new and removed fields in the foil model.
- pystormbird and stormbird_setup: the `RotatingCylinder` class was updated with the new lift parameters. The `rotor_sail_ostman_2023` and `rotor_sail_deybach_2024` models in stormbird_setup now use parameters fitted to the data in the respective papers.
- pystormbird: the `Foil` class has getters and setters for all fields, and the `RotatingCylinder` class has getters and setters for all fields, together with `__str__` and `__dict__`.
- pystormbird: invalid input when creating section models, simulations, line force models, complete sail models and wind environments now raises a `ValueError` rather than a panic. Reading simulation results from file raises an `OSError` if the file can not be read, and a `ValueError` if the content can not be parsed.
- stormbird_setup: new `far_field_ratio` and `initial_relative_wake_length` fields in the `DynamicWakeBuilder`, with the same defaults as in the Rust library.
- stormbird_setup: the `VaryingFoil` class has the same checks as in the Rust library. The `default_wing_sail_two_element` and `default_suction_sail_turbosail` models now give the foil data for positive internal states only, and use the mirroring in the Rust library for negative internal states. The two-element model gives the same result as before. The TurboSail model gives the same result as before, except for small negative power coefficients, between -0.1187 and 0, where the foils are now the exact mirror image of the foils for positive power coefficients.
- pystormbird: new `wake_as_vtk` function on the lifting line simulation, which works for both quasi-steady and dynamic wakes, and `induced_velocities` now also works for quasi-steady wakes. `dynamic_wake_as_vtk` is deprecated, gives a `DeprecationWarning`, and returns the same data as `wake_as_vtk`.
- stormbird_setup: the `write_wake_data_to_file` and `wake_files_folder_path` fields were removed from the `DynamicWakeBuilder`. The rotor sail and dynamic wake shape examples in pystormbird now write the wake files from the script, with `wake_as_vtk`.

### Bug fixes
- The input power model and the `RotatingCylinder` model in stormbird_setup did not match the Rust library, and could produce input that the Rust library would not accept.
- Setting up a rotor sail with the simplified setup in stormbird_setup failed, due to an unsupported argument to the controller set points.
- The initial shape of the dynamic wake, before the first time step, had a length proportional to the chord length squared, rather than the chord length, as the direction of the wake was not normalized. The wake is initialized again at the first time step, so this only affected the wake shape before the simulation started.

