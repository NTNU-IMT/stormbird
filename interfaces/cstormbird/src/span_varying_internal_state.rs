// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Shared implementation of the functions that set an internal state that varies along the span of
//! a wing, for both the complete sail model and the lifting line simulation.

use stormbird::error::Error;
use stormbird::line_force_model::LineForceModel;
use stormbird::line_force_model::wing_internal_state::{SpanVariation, WingInternalState};

use crate::error::{set_last_error, STORMBIRD_ERROR_INVALID_INPUT, STORMBIRD_ERROR_NULL_POINTER};
use crate::ffi_utils::read_floats;

/// Sets a span varying internal state on a wing, with checks of the input. Returns 0 on success,
/// -20 if a pointer is null, -3 if an array has zero length, and -3 if the wing index is out of
/// range or the span variation is not valid.
fn set_span_variation(
    function_name: &str,
    line_force_model: &mut LineForceModel,
    wing_index: usize,
    span_variation: Result<SpanVariation, Error>,
) -> i32 {
    if wing_index >= line_force_model.nr_wings() {
        set_last_error(STORMBIRD_ERROR_INVALID_INPUT, format!(
            "{}: wing index {} is out of range. The model has {} wings",
            function_name, wing_index, line_force_model.nr_wings()
        ));

        return -STORMBIRD_ERROR_INVALID_INPUT;
    }

    match span_variation {
        Ok(span_variation) => {
            line_force_model.set_wing_internal_state(
                wing_index, WingInternalState::SpanVarying(span_variation)
            );

            0
        },
        Err(error) => {
            set_last_error(error.code(), format!("{}: {}", function_name, error));

            -error.code()
        }
    }
}

/// Sets an internal state that is linearly interpolated between non-dimensional span locations.
/// See the public functions for the documentation of the arguments and return values.
pub(crate) fn set_internal_state_linear(
    function_name: &str,
    line_force_model: &mut LineForceModel,
    wing_index: usize,
    locations: *const f64,
    values: *const f64,
    length: usize,
) -> i32 {
    if locations.is_null() || values.is_null() {
        set_last_error(STORMBIRD_ERROR_NULL_POINTER, format!("{}: one of the input pointers is null", function_name));

        return -STORMBIRD_ERROR_NULL_POINTER;
    }

    if length == 0 {
        set_last_error(STORMBIRD_ERROR_INVALID_INPUT, format!("{}: the input arrays have zero length", function_name));

        return -STORMBIRD_ERROR_INVALID_INPUT;
    }

    let span_variation = SpanVariation::new_linear(
        read_floats(locations, length),
        read_floats(values, length)
    );

    set_span_variation(function_name, line_force_model, wing_index, span_variation)
}

/// Sets an internal state that is constant within zones along the span. See the public functions
/// for the documentation of the arguments and return values.
pub(crate) fn set_internal_state_piecewise_constant(
    function_name: &str,
    line_force_model: &mut LineForceModel,
    wing_index: usize,
    boundaries: *const f64,
    nr_boundaries: usize,
    values: *const f64,
    nr_values: usize,
) -> i32 {
    if boundaries.is_null() || values.is_null() {
        set_last_error(STORMBIRD_ERROR_NULL_POINTER, format!("{}: one of the input pointers is null", function_name));

        return -STORMBIRD_ERROR_NULL_POINTER;
    }

    if nr_boundaries == 0 || nr_values == 0 {
        set_last_error(STORMBIRD_ERROR_INVALID_INPUT, format!("{}: one of the input arrays has zero length", function_name));

        return -STORMBIRD_ERROR_INVALID_INPUT;
    }

    let span_variation = SpanVariation::new_piecewise_constant(
        read_floats(boundaries, nr_boundaries),
        read_floats(values, nr_values)
    );

    set_span_variation(function_name, line_force_model, wing_index, span_variation)
}
