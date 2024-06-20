// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use super::*;

/// Calculates how much the velocity is decreased due to the presence of a viscous wake.
/// Based on the model in <https://www.sciencedirect.com/science/article/pii/S0029801822010447#bib31>
pub fn schlichting_correction_factor(wake_coordinates: &WakeCoordinates, parameters: &WakeModelParameters) -> f64 {
    if parameters.drag_coefficient == 0.0 || parameters.width == 0.0 {
        return 1.0;
    }  
    
    let b = 1.14 * (parameters.drag_coefficient * parameters.width * wake_coordinates.velocity).sqrt();

    let y_term = (2.0 * wake_coordinates.normal / b).powf(1.5);

    if wake_coordinates.velocity <= 0.1 * parameters.width || y_term > 1.0 {
        1.0
    } else {
        let first_factor  = (wake_coordinates.velocity / (parameters.drag_coefficient * parameters.width)).powf(-0.5);
        let second_factor = (1.0 - y_term).powi(2);

        1.0 - 0.98 * first_factor * second_factor
    }
}