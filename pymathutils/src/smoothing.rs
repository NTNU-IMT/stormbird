use pyo3::prelude::*;

use math_utils::smoothing as smoothing_rust;

/// Formats the sum of two numbers as string.
#[pyfunction]
#[pyo3(
    signature = (
        *,
        x,
        y,
        smoothing_length,
        number_of_end_insertions,
        end_conditions
    )
)]
fn gaussian_smoothing(
    x: Vec<f64>, 
    y: Vec<f64>, 
    smoothing_length: f64, 
    number_of_end_insertions: usize, 
    end_conditions: [String; 2]
) -> PyResult<Vec<f64>> {
    let first_end_condition = smoothing_rust::SmoothingEndCondition::from_str(&end_conditions[0]);
    let second_end_condition = smoothing_rust::SmoothingEndCondition::from_str(&end_conditions[1]);

    let end_conditions = [first_end_condition, second_end_condition];
    
    Ok(
        smoothing_rust::gaussian_smoothing(
            &x, 
            &y, 
            smoothing_length,
            number_of_end_insertions,
            end_conditions
        )
    )
}

#[pyfunction]
#[pyo3(
    signature = (
        *,
        y,
        end_conditions,
        window_size
    )
)]
fn cubic_polynomial_smoothing(
    y: Vec<f64>,
    end_conditions: [String; 2],
    window_size: String
) -> PyResult<Vec<f64>> {
    let first_end_condition = smoothing_rust::SmoothingEndCondition::from_str(&end_conditions[0]);
    let second_end_condition = smoothing_rust::SmoothingEndCondition::from_str(&end_conditions[1]);

    let window_size = smoothing_rust::CubicPolynomialSmoothingWindowSize::from_str(&window_size);

    let end_conditions = [first_end_condition, second_end_condition];

    Ok(
        smoothing_rust::cubic_polynomial_smoothing(
            &y,
            end_conditions,
            window_size
        )
    )
}

#[pymodule]
pub fn smoothing(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(gaussian_smoothing, m)?)?;
    m.add_function(wrap_pyfunction!(cubic_polynomial_smoothing, m)?)?;

    Ok(())
}