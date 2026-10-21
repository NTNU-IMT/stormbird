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
- New structure for the `ActiveAirFlowFoil` section model, intended for suction sails. It now consists of a `VaryingFoil` model for the lift and drag, together with an `InternalStateConversion` that defines how the wing internal state is converted to the internal state of the foil model. Three conversions are available:
    - `SuctionCoefficientDirect`: the wing internal state is the suction coefficient, used directly. Mostly useful for testing.
    - `PowerToPowerCoefficient`: the wing internal state is the power, converted to a power coefficient on each line element. This is the same behavior as for the `ActiveAirFlowFoil` in earlier versions.
    - `PowerToInternalFlow`: the wing internal state is the fan input power, converted to a suction coefficient on each line element using a new, simplified model of the internal flow in the sail, including a constant fan efficiency.
- All section models can now compute a power coefficient. Foils return zero, the `ActiveAirFlowFoil` returns a value consistent with the power given as the wing internal state, and the `RotatingCylinder` uses a polynomial model, with default values from Tillig and Ringsberg (2020).
- New default input power model, `FromSectionModelPowerCoefficient`, which computes the power by integrating the power coefficient of the section models over the wing.
- The controller can now take the internal state set points either in the units of the section model (e.g., spin ratio or suction coefficient), or directly as the wing internal state (e.g., power), through the new `internal_state_set_point_type` setting.

### Breaking changes
- The input format for the `ActiveAirFlowFoil` section model has changed. The `VaryingFoil` data must now be given in a `varying_foil` field, together with an `internal_state_conversion` field. To get the same behavior as before, use the `PowerToPowerCoefficient` conversion.
- The default input power model has changed from `NoPower` to `FromSectionModelPowerCoefficient`. This gives the same result for wing sails, but rotor sails without an explicit input power model will now get the power from the polynomial power model in the `RotatingCylinder`. Set the input power model to `NoPower` explicitly to get the old behavior.
- Fields in the controller set points were renamed to reflect that the set points are no longer always given in the units of the section model, and that the limits are applied to the wing internal state:
    - `section_model_internal_state_data` -> `internal_state_data`
    - `internal_state_max_value` -> `wing_internal_state_max_value`
    - `max_internal_state_change_rate` -> `wing_internal_state_max_change_rate`
- The `input_power` function on the line force model now takes the sectional force input as an argument, as the power can depend on the local flow conditions.

### Changes to the Python library
- pystormbird: new `ActiveAirFlowFoil` class, and a new `power_coefficient` function for the `RotatingCylinder` class. The `as_active_air_flow_foil` function on the `VaryingFoil` class was removed, as an active air flow foil now also needs an internal state conversion.
- stormbird_setup: updated to the new data structures for the section models, input power models and controller set points. The input power model now matches the variants in the Rust library, and `new_polynomial_rotor_sail_model` gives the power directly, without the `area` argument. The suction sail in the simplified setup now uses the default input power model.

### Bug fixes
- The input power model and the `RotatingCylinder` model in stormbird_setup did not match the Rust library, and could produce input that the Rust library would not accept.
- Setting up a rotor sail with the simplified setup in stormbird_setup failed, due to an unsupported argument to the controller set points.

