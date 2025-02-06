use pyo3::prelude::*;

use pyo3:: wrap_pymodule;

mod smoothing;
mod spatial_vector;

use spatial_vector::SpatialVector;

/// A Python module implemented in Rust.
#[pymodule]
fn pymathutils(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<SpatialVector>()?;
    m.add_wrapped(wrap_pymodule!(smoothing::smoothing))?;
    
    Ok(())
}
