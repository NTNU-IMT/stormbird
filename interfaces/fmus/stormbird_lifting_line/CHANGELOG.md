# Change log Stormbird lifting line FMU

This file contains an overview of the changes made to the Stormbird lifting line FMU. See the [Stormbird change log](../../../stormbird/CHANGELOG.md) for changes to the underlying models. The version of the FMU follows the version of Stormbird.

## 0.13.0 - Unreleased
### Breaking changes
- With `motion_velocity_in_body_fixed_frame`, the angular motion velocity input is also given in the body fixed coordinate system, e.g., (p, q, r) from a ship simulator, and converted to the global coordinate system. Before, the setting only applied to the linear motion velocity, and the angular velocity input was always used as a global vector.
- The FMU is built around the `CompleteSailModel` in Stormbird, rather than separate models for the lifting line simulation, the wind environment and the controller. The model is set up from a single parameters file, where the complete sail model is given inline under the key `sail_model`, as the JSON representation of a `CompleteSailModelBuilder`. The separate setup files, and the fields `lifting_line_setup_file_path`, `wind_environment_setup_file_path` and `controller_setup_file_path` in the parameters file, are removed. The unused fields `superstructure_force_setup_path` and `non_dim_spanwise_measurement_position` are also removed. The spanwise position of the flow measurements is set with the spanwise measurement in the controller.
- Depends on Stormbird 0.13.0 and fmu_from_struct 0.3.0.
- The FMU is built and packaged with the `cargo fmu-from-struct` subcommand, which replaces the `package_fmu_after_build` tool. The FMU is written to `target/fmu/release/StormbirdLiftingLine.fmu`.

### Features
- The FMU can be built for version 3 of the FMI standard, in addition to version 2, by selecting the new cargo feature `fmi3` instead of the default `fmi2`, e.g., `cargo fmu-from-struct --release --no-default-features --features fmi3`. The variables and the behavior are the same for both versions.
- New `error_code` output. The FMU no longer crashes, or returns an error status to the importer, due to errors in the input or in the simulation, including panics. Instead, the error is reported as a non-zero error code, and the error message is printed to the standard error output. After an error, the model is not simulated anymore, and all other outputs are zero, until the FMU is restarted. See the README for the meaning of each code.
- The number of sails is checked when the model is set up. More than 10 sails gives an error code, rather than a panic in the first time step.
- A freestream velocity that is not finite, e.g., due to a NaN input, gives an error code. Before, it was treated as a zero input, which gave zero forces without a warning.
- The flow measurements are given as output also without a controller, using the default spanwise measurement.

### Bug fixes
- The angular velocity from finite differences of the rotation input was the time derivative of the rotation angles, which is wrong when more than one angle is different from zero, e.g., for a roll motion with a heading. The roll rate was then applied around the global x-axis rather than the x-axis of the ship. The angular velocity is now computed from the change in the rotation matrix, which is fixed in Stormath 0.4.1. This affects the velocity felt by the sails, the gyroscopic moment of rotor sails, and the measured apparent wind direction.
- A non-zero translation or rotation in the first time step, e.g., a ship position far from the origin or a heading, gave a large velocity from the finite differences, as the previous position was zero. The velocities are now zero in the first time step. The motion is also tracked while waiting for `number_of_iterations_before_building_model`, so the velocities are correct when the simulation starts.
- The sails were not simulated when the freestream velocity was zero, even if the ship was moving. With the default settings, where the ship velocity is given as motion of the sails, a moving ship in calm wind therefore gave zero forces. The check now uses the velocity felt by the sails, i.e., the freestream minus the motion of the sails.
- The inflow corrections were looked up with the true wind direction rather than the apparent wind direction, when the ship velocity was given as motion of the sails. This is fixed in the `CompleteSailModel` in Stormbird.
- With a controller and `number_of_iterations_before_building_model` larger than zero, the local wing angles and internal states from the inputs were never used as initial values. They are now used in the first time step where the model is simulated.
