// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use pyo3::prelude::*;
use pyo3::types::PyType;

use stormbird::common_utils::results::simulation::SimulationResult as SimulationResultRust;
use stormbird::common_utils::forces_and_moments::SectionalForces as SectionalForcesRust;
use stormbird::common_utils::forces_and_moments::IntegratedValues as IntegratedValuesRust;
use stormbird::common_utils::forces_and_moments::SectionalForcesInput as SectionalForcesInputRust;

#[pyclass]
#[derive(Clone)]
pub struct SectionalForcesInput {
    pub data: SectionalForcesInputRust
}

#[pymethods]
impl SectionalForcesInput {
    #[getter]
    pub fn circulation_strength(&self) -> Vec<f64> {
        self.data.circulation_strength.clone()
    }

    #[getter]
    pub fn velocity(&self) -> Vec<[f64; 3]> {
        self.data.velocity.iter().map(|v| v.0).collect()
    }

    #[getter]
    pub fn angles_of_attack(&self) -> Vec<f64> {
        self.data.angles_of_attack.clone()
    }

    #[getter]
    pub fn acceleration(&self) -> Vec<[f64; 3]> {
        self.data.acceleration.iter().map(|v| v.0).collect()
    }

    #[getter]
    pub fn rotation_velocity(&self) -> [f64; 3] {
        self.data.rotation_velocity.0
    }
}

#[pyclass]
#[derive(Clone)]
pub struct SectionalForces {
    pub data: SectionalForcesRust
}

#[pymethods]
impl SectionalForces {
    #[getter]
    pub fn circulatory(&self) -> Vec<[f64; 3]> {
        self.data.circulatory.iter().map(|v| v.0).collect()
    }

    #[getter]
    pub fn sectional_drag(&self) -> Vec<[f64; 3]> {
        self.data.sectional_drag.iter().map(|v| v.0).collect()
    }

    #[getter]
    pub fn added_mass(&self) -> Vec<[f64; 3]> {
        self.data.added_mass.iter().map(|v| v.0).collect()
    }

    #[getter]
    pub fn gyroscopic(&self) -> Vec<[f64; 3]> {
        self.data.gyroscopic.iter().map(|v| v.0).collect()
    }

    #[getter]
    pub fn total(&self) -> Vec<[f64; 3]> {
        self.data.total.iter().map(|v| v.0).collect()
    }
}

#[pyclass]
#[derive(Clone)]
pub struct IntegratedValues {
    pub data: IntegratedValuesRust
}

#[pymethods]
impl IntegratedValues {
    #[getter]
    pub fn circulatory(&self) -> [f64; 3] {
        self.data.circulatory.0
    }

    #[getter]
    pub fn sectional_drag(&self) -> [f64; 3] {
        self.data.sectional_drag.0
    }

    #[getter]
    pub fn added_mass(&self) -> [f64; 3] {
        self.data.added_mass.0
    }

    #[getter]
    pub fn gyroscopic(&self) -> [f64; 3] {
        self.data.gyroscopic.0
    }

    #[getter]
    pub fn total(&self) -> [f64; 3] {
        self.data.total.0
    }
}

#[pyclass]
#[derive(Clone)]
pub struct SimulationResult {
    pub data: SimulationResultRust
}

#[pymethods]
impl SimulationResult {
    #[classmethod]
    pub fn from_json_file(_cls: &Bound<'_, PyType>, file_path: String) -> SimulationResult {
        let data = SimulationResultRust::from_json(&file_path).unwrap();
        
        SimulationResult { data }
    }
    
    #[classmethod]
    pub fn result_history_from_file(_cls: &Bound<'_, PyType>, file_path: String) -> Vec<SimulationResult> {
        let rust_vector = SimulationResultRust::result_history_from_file(&file_path).unwrap();

        rust_vector.iter().map(|v| SimulationResult { data: v.clone() }).collect()
    }

    #[getter]
    pub fn ctrl_points(&self) -> Vec<[f64; 3]> {
        self.data.ctrl_points.iter().map(|v| v.0).collect()
    }

    #[getter]
    pub fn force_input(&self) -> SectionalForcesInput {
        SectionalForcesInput { data: self.data.force_input.clone() }
    }

    #[getter]
    pub fn sectional_forces(&self) -> SectionalForces {
        SectionalForces { data: self.data.sectional_forces.clone() }
    }

    #[getter]
    pub fn integrated_forces(&self) -> Vec<IntegratedValues> {
        self.data.integrated_forces.iter().map(|v| IntegratedValues { data: v.clone() }).collect()
    }

    #[getter]
    pub fn integrated_moments(&self) -> Vec<IntegratedValues> {
        self.data.integrated_moments.iter().map(|v| IntegratedValues { data: v.clone() }).collect()
    }

    #[getter]
    pub fn input_power(&self) -> Vec<f64> {
        self.data.input_power.clone()
    }

    #[getter]
    pub fn iterations(&self) -> usize {
        self.data.iterations
    }

    #[getter]
    pub fn residual(&self) -> f64 {
        self.data.residual
    }
    
    #[getter]
    pub fn wing_indices(&self) -> Vec<[usize; 2]> {
        let mut out = Vec::with_capacity(self.data.nr_of_wings());
        
        for i in 0..self.data.nr_of_wings() {
            out.push(
                [self.data.wing_indices[i].start, self.data.wing_indices[i].end]
            )
        }
        
        out
    }

    fn __str__(&self) -> String {
        format!("{:?}", self.data)
    }

    pub fn integrated_forces_sum(&self) -> [f64; 3] {
        self.data.integrated_forces_sum().0
    }

    pub fn integrated_moments_sum(&self) -> [f64; 3] {
        self.data.integrated_moments_sum().0
    }

    pub fn input_power_sum(&self) -> f64 {
        self.data.input_power.iter().sum()
    }
}
