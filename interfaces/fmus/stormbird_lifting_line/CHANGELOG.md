# Change log Stormbird lifting line FMU

This file contains an overview of the changes made to the Stormbird lifting line FMU. See the [Stormbird change log](../../../stormbird/CHANGELOG.md) for changes to the underlying models. The version of the FMU follows the version of Stormbird.

## 0.13.0 - Unreleased
### Breaking changes
- The FMU is built around the `CompleteSailModel` in Stormbird, rather than separate models for the lifting line simulation, the wind environment and the controller. The model is set up from a single parameters file, where the complete sail model is given inline under the key `sail_model`, as the JSON representation of a `CompleteSailModelBuilder`. The separate setup files, and the fields `lifting_line_setup_file_path`, `wind_environment_setup_file_path` and `controller_setup_file_path` in the parameters file, are removed. The unused fields `superstructure_force_setup_path` and `non_dim_spanwise_measurement_position` are also removed. The spanwise position of the flow measurements is set with the spanwise measurement in the controller.
- Depends on Stormbird 0.13.0 and fmu_from_struct 0.3.0.
- The FMU is built and packaged with the `cargo fmu-from-struct` subcommand, which replaces the `package_fmu_after_build` tool. The FMU is written to `target/fmu/release/StormbirdLiftingLine.fmu`.

### Features
- New `error_code` output. The FMU no longer crashes, or returns an error status to the importer, due to errors in the input or in the simulation, including panics. Instead, the error is reported as a non-zero error code, and the error message is printed to the standard error output. After an error, the model is not simulated anymore, and all other outputs are zero, until the FMU is restarted. See the README for the meaning of each code.
- The number of sails is checked when the model is set up. More than 10 sails gives an error code, rather than a panic in the first time step.
- A freestream velocity that is not finite, e.g., due to a NaN input, gives an error code. Before, it was treated as a zero input, which gave zero forces without a warning.
- The flow measurements are given as output also without a controller, using the default spanwise measurement.

### Bug fixes
- With a controller and `number_of_iterations_before_building_model` larger than zero, the local wing angles and internal states from the inputs were never used as initial values. They are now used in the first time step where the model is simulated.
