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
