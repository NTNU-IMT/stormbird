// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

/// A function that returns a vector that linearly goes from a start value to an end value with a
/// specified number of elements.
///
/// # Arguments
/// * `x_start` - the value of the first element
/// * `x_end` - the value of the last element
/// * `n` - the number of elements
pub fn linspace(x_start: f64, x_end: f64, n: usize) -> Vec<f64> {
    let dx = (x_end - x_start) / ((n - 1) as f64);

    let mut x: Vec<f64> = Vec::new();

    let mut x_current = x_start;

    for _ in 0..n {
        x.push(x_current);

        x_current += dx;
    }

    x
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linspace_test() {
        let allowable_error = 1e-6;

        let x_start = 2.3456;
        let x_end = 12.4535653;

        let n: usize = 103;

        let x = linspace(x_start, x_end, n);

        println!("first value of x: {}", x[0]);
        println!("Last value of x: {}", x.last().unwrap());

        assert!((x[0] - x_start).abs() < allowable_error);
        assert!((x.last().unwrap() - x_end).abs() < allowable_error);
        assert_eq!(x.len(), n);
    }
}