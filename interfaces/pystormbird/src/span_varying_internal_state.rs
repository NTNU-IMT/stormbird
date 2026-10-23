// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Helper functions for setting internal states that vary along the span of a wing.

use pyo3::prelude::*;
use pyo3::exceptions::{PyValueError, PyIndexError};

use stormbird::line_force_model::LineForceModel;
use stormbird::line_force_model::wing_internal_state::{SpanVariation, WingInternalState};

/// Sets a span varying internal state on a wing in the line force model, with checks of the input
fn set_span_variation(
    line_force_model: &mut LineForceModel,
    wing_index: usize,
    span_variation: Result<SpanVariation, String>,
) -> PyResult<()> {
    if wing_index >= line_force_model.nr_wings() {
        return Err(PyIndexError::new_err(format!(
            "Wing index {} is out of range. The model has {} wings",
            wing_index, line_force_model.nr_wings()
        )));
    }

    let span_variation = span_variation.map_err(PyValueError::new_err)?;

    line_force_model.set_wing_internal_state(
        wing_index, WingInternalState::SpanVarying(span_variation)
    );

    Ok(())
}

/// Sets an internal state that is linearly interpolated between non-dimensional span locations
pub fn set_internal_state_linear(
    line_force_model: &mut LineForceModel,
    wing_index: usize,
    locations: Vec<f64>,
    values: Vec<f64>,
) -> PyResult<()> {
    set_span_variation(line_force_model, wing_index, SpanVariation::new_linear(locations, values))
}

/// Sets an internal state that is constant within zones along the span
pub fn set_internal_state_piecewise_constant(
    line_force_model: &mut LineForceModel,
    wing_index: usize,
    boundaries: Vec<f64>,
    values: Vec<f64>,
) -> PyResult<()> {
    set_span_variation(
        line_force_model, wing_index, SpanVariation::new_piecewise_constant(boundaries, values)
    )
}
