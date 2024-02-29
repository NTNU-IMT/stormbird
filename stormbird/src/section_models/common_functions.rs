// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

pub fn sigmoid_function(x: f64, x0: f64, transition_range: f64) -> f64 {
    let slope = 4.59512 / transition_range;

    let x_prime = slope * (x - x0);

    1.0 / ( 1.0 + f64::exp(-x_prime))
}