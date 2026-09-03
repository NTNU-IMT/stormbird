
use pyo3::prelude::*;
use pyo3::types::PyBytes;

use stormath::type_aliases::Float;
use stormflow::simulation::Simulation as SimulationRust;

use crate::result_structs::SimulationResult;

#[pyclass]
pub struct Simulation {
    data: SimulationRust
}

#[pymethods]
impl Simulation {
    #[new]
    pub fn new(setup_string: String) -> Self {
        Self {
            data: SimulationRust::new_from_string(&setup_string).unwrap()
        }
    }

    pub fn time_step_from_courant_number(&self, courant_number: Float) -> Float {
        self.data.time_step_from_courant_number(courant_number)
    }

    pub fn do_step(&mut self, time: Float, time_step: Float) {
        self.data.do_step(time, time_step);
    }

    pub fn do_steps_until_end_time(&mut self, end_time: Float, courant_number: Float) {
        self.data.do_steps_until_end_time(end_time, courant_number);
    }

    pub fn export_fields_as_vtk(&self, file_path: String, binary: bool) {
        self.data.export_fields_as_vtk(&file_path, binary);
    }

    /// Return the VTK legacy file content for the current fields as raw bytes,
    /// without touching the disk.
    ///
    /// The `binary` switch selects between the ASCII and BINARY variants of the
    /// VTK legacy format. The returned `bytes` can be fed straight into a VTK
    /// reader to build a mesh in memory, e.g. with pyvista:
    ///
    ///     import vtk
    ///     import pyvista as pv
    ///
    ///     data = simulation.fields_as_vtk(binary=True)
    ///
    ///     reader = vtk.vtkStructuredGridReader()
    ///     reader.ReadFromInputStringOn()
    ///     reader.SetBinaryInputString(data, len(data))  # use SetInputString for ASCII
    ///     reader.ReadAllScalarsOn()
    ///     reader.ReadAllVectorsOn()
    ///     reader.Update()
    ///
    ///     grid = pv.wrap(reader.GetOutput())
    pub fn fields_as_vtk<'py>(&self, py: Python<'py>, binary: bool) -> Bound<'py, PyBytes> {
        let content = self.data.fields_as_vtk(binary);
        PyBytes::new(py, &content)
    }

    pub fn get_stormbird_simulation_result(&self) -> Option<SimulationResult> {
        if let Some(actuator_line) = &self.data.actuator_line {
            if let Some(result) = &actuator_line.model.simulation_result {
                Some(
                    SimulationResult{
                        data: result.clone()
                    }
                )
            } else {
                None
            }
        } else {
            None
        }
    }
}