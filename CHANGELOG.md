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