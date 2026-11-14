# Change log cstormbird

This file contains an overview of the changes made to cstormbird, the C interface to the Stormbird library. The version number of cstormbird follows the version number of the Stormbird Rust library. See the [Stormbird change log](../../stormbird/CHANGELOG.md) for changes to the underlying models.

## 0.13.0 - 2026-11-14
- Depends on the published versions of Stormbird 0.13.0 and Stormath 0.5.0. There are no changes to the C interface itself. See the [Stormbird change log](../../stormbird/CHANGELOG.md) for the changes to the underlying models, which also cover the changes in Stormbird 0.12.0. The most important ones for users of the C interface are:
    - Invalid settings in the setup strings, e.g., a wing with fewer than two section points or a line force model without wings, are reported as errors when a model is created, with a description from `stormbird_last_error_message`, rather than as a panic or invalid results later.
    - The controller in the complete sail model is optional, and the steady state simulations no longer depend on earlier calls or on the settings of the controller that are only meaningful in the time domain.
    - The angular velocity computed with `lifting_line_simulation_set_translation_and_rotation_with_finite_difference_for_the_velocity` is correct also when more than one rotation angle is different from zero.
    - The default `wind_rotation_axis` in the wind environment is the negative z axis since Stormbird 0.12.0, so the wind direction is measured clockwise seen from above.

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
