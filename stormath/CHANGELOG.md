# Change log Stormath

This file contains an overview of the changes made to Stormath, the mathematical utilities used by Stormbird.

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
