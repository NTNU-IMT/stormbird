use pyo3::prelude::*;

use math_utils::smoothing::end_condition::EndCondition;
use math_utils::smoothing::gaussian as gaussian_smoothing_rust;
use math_utils::smoothing::polynomial as polynomial_smoothing_rust;

/// Formats the sum of two numbers as string.
#[pyfunction]
#[pyo3(
    signature = (
        *,
        x,
        y,
        smoothing_length,
        end_conditions
    )
)]
fn gaussian_smoothing(
    x: Vec<f64>, 
    y: Vec<f64>, 
    smoothing_length: f64,
    end_conditions: [String; 2]
) -> PyResult<Vec<f64>> {
    let first_end_condition = EndCondition::from_str(&end_conditions[0]);
    let second_end_condition = EndCondition::from_str(&end_conditions[1]);

    let end_conditions = [first_end_condition, second_end_condition];

    let gaussian_smoothing = gaussian_smoothing_rust::GaussianSmoothing {
        smoothing_length,
        number_of_end_insertions: None,
        end_conditions
    };
    
    Ok(
        gaussian_smoothing.apply_smoothing(&x, &y)
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
    let first_end_condition = EndCondition::from_str(&end_conditions[0]);
    let second_end_condition = EndCondition::from_str(&end_conditions[1]);

    let window_size = polynomial_smoothing_rust::WindowSize::from_str(&window_size);

    let end_conditions = [first_end_condition, second_end_condition];

    let cubic_polynomial_smoothing = polynomial_smoothing_rust::CubicPolynomialSmoothing {
        window_size,
        end_conditions
    };

    Ok(
        cubic_polynomial_smoothing.apply_smoothing(&y)
    )
}

#[pymodule]
pub fn smoothing(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(gaussian_smoothing, m)?)?;
    m.add_function(wrap_pyfunction!(cubic_polynomial_smoothing, m)?)?;

    Ok(())
}