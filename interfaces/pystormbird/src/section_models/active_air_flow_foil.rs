// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use pyo3::prelude::*;
use pyo3::exceptions::PyValueError;
use pythonize::pythonize;

use stormbird::section_models::active_air_flow_foil::{
    ActiveAirFlowFoil as ActiveAirFlowFoilRust,
    SegmentFlowConditions
};
use stormbird::section_models::SectionModel as SectionModelRust;
use stormbird::line_force_model::wing_internal_state::{WingInternalState, InternalCompartments};

use super::SectionModel;

#[pyclass(from_py_object)]
#[derive(Clone)]
pub struct ActiveAirFlowFoil {
    pub data: ActiveAirFlowFoilRust
}

#[pymethods]
impl ActiveAirFlowFoil {
    #[new]
    pub fn new(input_str: String) -> PyResult<Self> {
        let data = ActiveAirFlowFoilRust::new_from_string(&input_str).map_err(|e| {
            PyValueError::new_err(format!("Failed to parse active air flow foil from string: {}", e))
        })?;

        Ok(Self { data })
    }

    pub fn lift_coefficient(&self, angle_of_attack: f64, internal_state: f64) -> f64 {
        self.data.lift_coefficient(angle_of_attack, internal_state)
    }

    pub fn drag_coefficient(&self, angle_of_attack: f64, internal_state: f64) -> f64 {
        self.data.drag_coefficient(angle_of_attack, internal_state)
    }

    pub fn power_coefficient(&self, angle_of_attack: f64, internal_state: f64) -> f64 {
        self.data.power_coefficient(angle_of_attack, internal_state)
    }

    /// Converts the internal state set on the wing to the internal state of the foil model on each
    /// segment. The three input lists must have the same length, with one value per segment.
    pub fn section_internal_states(
        &self,
        wing_state: f64,
        angles_of_attack: Vec<f64>,
        velocities: Vec<f64>,
        areas: Vec<f64>,
        density: f64
    ) -> PyResult<Vec<f64>> {
        let segments = Self::segments(&angles_of_attack, &velocities, &areas)?;

        Ok(self.data.section_internal_states(
            &WingInternalState::Constant(wing_state), &segments, &InternalCompartments::default(), density
        ))
    }

    /// Computes the power needed to operate the wing, from the wing state and the internal state
    /// of the foil model on each segment. All input lists must have the same length, with one 
    /// value per segment.
    pub fn wing_power(
        &self,
        wing_state: f64,
        angles_of_attack: Vec<f64>,
        velocities: Vec<f64>,
        areas: Vec<f64>,
        section_states: Vec<f64>,
        density: f64
    ) -> PyResult<f64> {
        let segments = Self::segments(&angles_of_attack, &velocities, &areas)?;

        if section_states.len() != segments.len() {
            return Err(PyValueError::new_err(
                "section_states must have the same length as the other input lists"
            ));
        }

        Ok(self.data.wing_power(
            &WingInternalState::Constant(wing_state), 
            &segments, 
            &InternalCompartments::default(), 
            &section_states, 
            density
        ))
    }

    /// Estimates the wing state that gives a specified section internal state, assuming uniform
    /// flow conditions over the wing.
    pub fn wing_state_from_section_state(
        &self,
        section_state: f64,
        angle_of_attack: f64,
        velocity: f64,
        wing_area: f64,
        density: f64
    ) -> f64 {
        self.data.wing_state_from_section_state(
            section_state, angle_of_attack, velocity, wing_area, 1, density
        )
    }

    pub fn as_section_model(&self) -> SectionModel {
        SectionModel {
            data: SectionModelRust::ActiveAirFlowFoil(self.data.clone())
        }
    }

    pub fn __str__(&self) -> String {
        self.data.to_string()
    }

    #[getter]
    /// Uses the built in json module to convert the string to a dictionary
    pub fn __dict__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        Ok(pythonize(py, &self.data)?)
    }
}

impl ActiveAirFlowFoil {
    fn segments(
        angles_of_attack: &[f64],
        velocities: &[f64],
        areas: &[f64]
    ) -> PyResult<Vec<SegmentFlowConditions>> {
        if angles_of_attack.len() != velocities.len() || angles_of_attack.len() != areas.len() {
            return Err(PyValueError::new_err(
                "angles_of_attack, velocities and areas must have the same length"
            ));
        }

        Ok((0..angles_of_attack.len()).map(|i| {
            // The span length and location are only used for wing states that vary along the 
            // span, which are not supported through this interface
            SegmentFlowConditions {
                angle_of_attack: angles_of_attack[i],
                velocity: velocities[i],
                area: areas[i],
                span_length: areas[i],
                non_dim_span_location: 0.0,
                is_virtual: false,
                compartment_index: 0,
            }
        }).collect())
    }
}
