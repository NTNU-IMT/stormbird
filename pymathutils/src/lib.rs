use pyo3::prelude::*;

use pyo3:: wrap_pymodule;

mod smoothing;


/// A Python module implemented in Rust.
#[pymodule]
fn pymathutils(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_wrapped(wrap_pymodule!(smoothing::smoothing))?;
    
    Ok(())
}
