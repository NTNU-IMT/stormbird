# Change log cstormbird

This file contains an overview of the changes made to cstormbird, the C interface to the Stormbird library. The version number of cstormbird follows the version number of the Stormbird Rust library. See the [Stormbird change log](../../stormbird/CHANGELOG.md) for changes to the underlying models.

## 0.13.0 - 2026-11-14
### Breaking changes
- Depends on the published versions of Stormbird 0.13.0 and Stormath 0.5.0. See the [Stormbird change log](../../stormbird/CHANGELOG.md) for the changes to the underlying models, which also cover the changes in Stormbird 0.12.0. The most important ones for users of the C interface are:
    - Invalid settings in the setup strings, e.g., a wing with fewer than two section points or a line force model without wings, are reported as errors when a model is created, with a description from `stormbird_last_error_message`, rather than as a panic or invalid results later.
    - The controller in the complete sail model is optional, and the steady state simulations no longer depend on earlier calls or on the settings of the controller that are only meaningful in the time domain.
    - The angular velocity computed with `lifting_line_simulation_set_translation_and_rotation_with_finite_difference_for_the_velocity` is correct also when more than one rotation angle is different from zero.
    - The default `wind_rotation_axis` in the wind environment is the negative z axis since Stormbird 0.12.0, so the wind direction is measured clockwise seen from above.

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
- New error code `-6` for input values that are not valid, e.g., a wing index that is out of range, or span locations that are not increasing.
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
