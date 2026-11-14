# Change log Stormath

This file contains an overview of the changes made to Stormath, the mathematical utilities used by Stormbird.

## 0.5.0 - 2026-11-14
### Breaking changes
- The `YawPitchRoll` variant of `RotationType` and the `SpatialVector::rotate_yaw_pitch_roll` function are removed. The rotation was by definition the same as `XYZ`, as yaw, pitch and roll around the rotated axes is the same as rotations around the fixed x, y and z axes, in that order. Use `XYZ` instead. JSON input with `"YawPitchRoll"` gives an error.

### Features
- New rotation types: `XZY`, `YXZ`, `YZX`, `ZXY` and `ZYX`, in addition to `XYZ`. The rotation vector always contains the angles around the x, y and z axes, while the rotation type defines the order in which the rotations around the fixed axes are applied. New `RotationType::axis_order` function, which returns this order, and new `SpatialVector::rotate_around_coordinate_axis` function, which rotates a vector around a single coordinate axis.
- New `angular_velocity_from_rotation_change` function in the `rigid_body_motion` module, which computes the angular velocity vector from the change in rotation during a time step, see the bug fix below.
- New `RigidBodyMotion::vector_from_body_fixed_to_global_coordinate_system` function, which is the inverse of `vector_in_body_fixed_coordinate_system`. It can, for instance, be used to convert an angular velocity in body fixed axes to the global angular velocity vector.

### Bug fixes
- `RigidBodyMotion::update_rotation_with_velocity_using_finite_difference` gave a wrong angular velocity when more than one rotation angle was different from zero. The angular velocity was computed as the time derivative of the rotation angles, which is only the angular velocity vector when the body rotates around a single axis. For instance, with a heading of 90 degrees, a roll rate was applied around the global x-axis rather than around the x-axis of the body, which then points along the global y-axis. The angular velocity is now computed from the rotation matrices, as the rotation around a single axis that takes the body from the old to the new rotation, divided by the time step. This is exact for a constant angular velocity during the time step, works for all rotation types, and handles angles that jump between -π and π, which was handled separately before. The results are unchanged for rotations around a single axis.

## 0.4.0 - 2026-10-23
### Highlights
- New bracketed root-finding method (Brent's method), through the `brent_solver` function. Used by Stormbird to find the operating point of fans. The solver returns an error if the function returns NaN, rather than continuing with an invalid value.

### Breaking changes
- `Matrix::linear_residual_absolute_sum` was renamed to `Matrix::linear_residual_absolute_mean`, as it returns the mean of the absolute values of the residual, not the sum. The returned value is unchanged.
- `EndCondition` and `WindowSize` in the smoothing module now implement the standard `FromStr` trait, so that they can be created with `parse`, and return an error for unknown names rather than panicking. The previous `from_str` methods, which panicked for unknown names, were removed.
- New `InvalidInput` variant in the `Error` enum, used for invalid input such as unknown names. The `Error` enum is now marked as `#[non_exhaustive]`, so that new variants can be added later without breaking changes. Matches on the enum must therefore include a wildcard arm.
- The `velocity_scale` function on the `ParticleSwarm` was removed, as it is no longer used in the velocity update.
- The minimum supported Rust version is now set to 1.85, through the `rust-version` field, as required by the `rand` dependency.

### Bug fixes
- `RigidBodyMotion::update_rotation_with_velocity_using_finite_difference` gave a wrong angular velocity when a rotation angle passed -π in the negative direction (i.e., jumped from close to -π to close to π), as `TAU` was added to the rotation difference rather than subtracted. Rotations passing π in the positive direction were handled correctly.
- The velocity update in the `ParticleSwarm` optimizer multiplied the velocity towards the local and global best positions with the range of the search domain, which made the particles overshoot and end up at the bounds when the range was larger than one. The velocity is no longer scaled.

### Other changes
- Updated the `rand` dependency to version 0.10. It is only used internally, in the `ParticleSwarm` optimizer.
- Added `repository`, `keywords` and `categories` to the crate metadata, and changed the license identifier from the deprecated `GPL-3.0` to `GPL-3.0-only`.
- Added documentation to all public modules, structures, fields and functions that were missing it.
- Fixed the documentation of `area_of_quadrilateral`, which described a different function.
- Fixed the formatting of lists in the documentation, and some typos in the documentation and README.
- Internal cleanup based on Clippy suggestions, without changes to the behavior.
- Fixed tests that failed or gave warnings with the `single_precision` feature, and added tests for the `ParticleSwarm` optimizer, for rotations that pass ±π in `RigidBodyMotion`, for NaN handling in `brent_solver` and for parsing of the smoothing settings.

## 0.3.0 and earlier
Changes up to and including version 0.3.0 were mainly made to support new functionality in the Stormbird Rust library.
