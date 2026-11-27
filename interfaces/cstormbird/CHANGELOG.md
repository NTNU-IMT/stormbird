# Change log cstormbird

This file contains an overview of the changes made to cstormbird, the C interface to the Stormbird library. The version number of cstormbird follows the version number of the Stormbird Rust library. See the [Stormbird change log](../../stormbird/CHANGELOG.md) for changes to the underlying models.

## 0.14.0 - 2026-11-27
### Breaking changes
- Depends on the published versions of Stormbird 0.14.0 and Stormath 0.5.1. See the [Stormbird change log](../../stormbird/CHANGELOG.md) for details. The changes that matter most for the setup strings and the results are:
    - The `velocity_corrections` field in the solvers is replaced by `induced_velocities`. `"velocity_corrections": X` becomes `"induced_velocities": {"total_correction": X}`, which gives the same results. `neglect_self_induced_velocities` is moved from the dynamic wake to the same structure.
    - More input structures reject unknown fields, so input with misspelled or extra fields gives an error rather than being ignored.
    - Spin ratio set points for rotor sails in the controller (`InternalStateSetPointType::SectionModel`, the default) must change sign to give the same results as before, as the conversion to revolutions per second is fixed. The default sails in stormbird_setup 0.14.0 are updated.
    - Bug fixes that change the results, e.g., the angle of attack for velocities along the span (heeled or rolling sails) and for chord vectors that are not normal to the span, the direction of the viscous lift with rotation, the averaging of the apparent wind direction in the controller across ±180 degrees, and the controller rate limits with `time_steps_between_updates` larger than one.
- `lifting_line_simulation_set_velocity_linear` and `lifting_line_simulation_set_velocity_angular` take a new `bool in_body_fixed_frame` argument, which specifies the coordinate system of the input velocity. With `true`, the velocity is given in the body fixed coordinate system, e.g., (u, v, w) and (p, q, r) from a ship simulator, and converted to the global coordinate system with the current rotation, so the rotation must be set first. With `false`, the velocity is used as a global vector, which is the same as before. Before, the input was always used as a global vector, and body fixed velocities gave wrong results when the sails were rotated, e.g., with a heading. Existing calls must add `false` as the last argument to keep the same behavior. The coordinate system of the input is independent of the `output_coordinate_system` of the line force model.

### Bug fixes
- Ten functions of the `LiftingLineSimulation` returned `-1` for a null pointer, rather than `-20` as documented and as stated in the change log for 0.13.0: `lifting_line_simulation_get_number_of_sails`, `_get_number_of_span_lines`, `_get_number_of_freestream_velocity_points`, `_has_dynamic_wake`, `_set_translation_only`, `_set_rotation_only`, `_set_translation_with_velocity_using_finite_difference`, `_set_rotation_with_velocity_using_finite_difference`, `_set_velocity_linear` and `_set_velocity_angular`. They now return `-20`. The error code and message stored for `stormbird_last_error_code` and `stormbird_last_error_message` were already correct.

## 0.13.1 - 2026-11-20
### Changes
- Depends on the published version of Stormbird 0.13.1, which contains bug fixes and one small feature. See the [Stormbird change log](../../stormbird/CHANGELOG.md) for details. The C functions and the header are unchanged. The changes that are relevant for users of the C interface are:
    - The setup strings for the line force model accept the new `internal_states` field, with one initial internal state for each wing, in the same way as `local_wing_angles`.
    - The rate limit of the local wing angles in the controller (`max_local_wing_angle_change_rate`) works correctly when the wing angle crosses pi. Before, the wings could be turned the wrong way and get stuck, typically with stalled sails.
    - Suction sails with the `PowerToInternalFlow` conversion no longer panic for very small, but non-zero, powers.

## 0.13.0 - 2026-11-14
### Breaking changes
- Depends on the published versions of Stormbird 0.13.0 and Stormath 0.5.0. See the [Stormbird change log](../../stormbird/CHANGELOG.md) for the changes to the underlying models, which also cover the changes in Stormbird 0.12.0. The most important ones for users of the C interface are:
    - Invalid settings in the setup strings, e.g., a wing with fewer than two section points or a line force model without wings, are reported as errors when a model is created, with a description from `stormbird_last_error_message`, rather than as a panic or invalid results later.
    - The controller in the complete sail model is optional, and the steady state simulations no longer depend on earlier calls or on the settings of the controller that are only meaningful in the time domain.
    - The angular velocity computed with `lifting_line_simulation_set_translation_and_rotation_with_finite_difference_for_the_velocity` is correct also when more than one rotation angle is different from zero.
    - The default `wind_rotation_axis` in the wind environment is the negative z axis since Stormbird 0.12.0, so the wind direction is measured clockwise seen from above.

- The error codes are the same as in the rest of Stormbird, e.g., the `error_code` output of the FMU: 1 for IO errors, 2 for JSON errors, 3 for invalid input, and 4 for other errors from Stormbird, as given by `Error::code` in Stormbird. The codes 20 (null pointer) and 21 (unexpected error, i.e., a panic that was caught) are specific to the C interface. Functions that return an `int32_t` return the negative of the code on failure. The old codes are mapped as follows:
    - `-1` (null pointer) is now `-20`.
    - `-2` (wrong array length), `-4` (wrong velocity variation) and the invalid UTF-8 part of `-3` are now `-3` (invalid input).
    - Invalid JSON, which was part of `-3`, is now `-2`.
    - Functions that returned `-1` for a null pointer in place of a count or an integer value, e.g., `complete_sail_model_get_number_of_sails`, now return `-20`.
  Errors from Stormbird, e.g., when a model is created, keep the code from Stormbird, which is available through `stormbird_last_error_code`.

### Features
The C interface is aligned with pystormbird 0.13.0, with the following new functions:
- `CompleteSailModel`:
    - `complete_sail_model_has_controller`, as the controller is optional.
    - `complete_sail_model_ship_velocity_vector`, and versions of the steady state simulations, the time step and the freestream velocity where the ship velocity is given as a vector: `complete_sail_model_simulate_steady_state_condition_with_ship_velocity_vector`, `complete_sail_model_simulate_optimal_steady_state_condition_with_ship_velocity_vector`, `complete_sail_model_do_step_with_ship_velocity_vector` and `complete_sail_model_freestream_velocity_with_ship_velocity_vector`.
    - `complete_sail_model_set_internal_state_linear` and `complete_sail_model_set_internal_state_piecewise_constant`, for internal states that vary along the span of a sail.
- `LiftingLineSimulation`: `lifting_line_simulation_set_internal_state_linear` and `lifting_line_simulation_set_internal_state_piecewise_constant`.
- `SimulationResult`:
    - `simulation_result_new_from_json_file`, which reads a result from file.
    - `simulation_result_nr_of_span_lines`, `simulation_result_iterations`, `simulation_result_residual` and `simulation_result_wing_indices`.
    - `simulation_result_integrated_forces`, `simulation_result_integrated_moments` and `simulation_result_input_power`, with the values for each sail. The forces and moments are written as the new `IntegratedValues` struct, with the different contributions.
    - `simulation_result_ctrl_points` and `simulation_result_sectional_forces`, with the forces on each line element as the new `SectionalForces` struct.
    - `simulation_result_force_input_circulation_strength`, `_velocity`, `_angles_of_attack`, `_section_models_internal_state`, `_amount_of_stall`, `_acceleration` and `_rotation_velocity`, with the input used to compute the forces.
- New `stormbird_last_error_code` function, which returns the code of the last failure, and constants for the error codes in the header: `STORMBIRD_OK`, `STORMBIRD_ERROR_IO`, `STORMBIRD_ERROR_JSON`, `STORMBIRD_ERROR_INVALID_INPUT`, `STORMBIRD_ERROR_OTHER`, `STORMBIRD_ERROR_NULL_POINTER` and `STORMBIRD_ERROR_UNEXPECTED`.
- The README describes all the functions, and the parts of pystormbird that are not available in the C interface.

## 0.11.0 - 2026-10-23
### Breaking changes
- Depends on the published versions of Stormbird 0.11.0 and Stormath 0.4.0. The input format for the Stormbird models has changed in Stormbird 0.11.0, so setup strings may need to be updated. For instance, the `Foil` section model is now called `SingleElementFoil`.
- The functions for the internal state are renamed, as they set and get the internal state of each sail, rather than the internal state of the section models:
    - `lifting_line_simulation_section_models_internal_state` -> `lifting_line_simulation_internal_state`
    - `lifting_line_simulation_set_section_models_internal_state` -> `lifting_line_simulation_set_internal_state`
    - `complete_sail_model_section_models_internal_state` -> `complete_sail_model_internal_state`
    - `complete_sail_model_set_section_models_internal_state` -> `complete_sail_model_set_internal_state`
- `lifting_line_simulation_induced_velocities` now also works for simulations with a quasi-steady wake, rather than returning the error code `-5`. The error code `-5` is no longer used.

### Other changes
- The internal state that is returned for a sail with an internal state that varies along the span is the mean value over the span.
- New tests for the induced velocities with a quasi-steady wake, and for setting and getting the internal state.

## 0.1.0
The first version.
