// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Handling of the ends of the data in smoothing functions.

use super::*;

use std::str::FromStr;

use crate::error::Error;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
/// Enum used to decide how to modify input data used in smoothing functions. 
pub enum EndCondition<T: SmoothingOps> {
    /// Pads the ends with zeros
    Zero,
    /// Pads the ends with a given value
    Given(T),
    /// Pads the ends with the same value as the end values
    Extended
}

impl<T: SmoothingOps> FromStr for EndCondition<T> {
    type Err = Error;

    /// Creates an end condition from its name. Only `Zero` and `Extended` are supported, as
    /// `Given` also needs a value.
    fn from_str(end_condition: &str) -> Result<Self, Self::Err> {
        match end_condition {
            "Zero" => Ok(Self::Zero),
            "Extended" => Ok(Self::Extended),
            _ => Err(Error::InvalidInput(
                format!("unknown smoothing end condition: {}", end_condition)
            ))
        }
    }
}

impl<T: SmoothingOps> EndCondition<T> {
    /// Returns the values to insert before the start of the data
    pub fn y_start_values(&self, y: &[T], number_of_end_insertions: usize) -> Vec<T> {
        let mut y_start: Vec<T> = Vec::with_capacity(number_of_end_insertions + 1);

        match self {
            Self::Zero => {
                for _ in 0..number_of_end_insertions {
                    y_start.push(T::default());
                }
            }
            Self::Given(value) => {
                for _ in 0..number_of_end_insertions {
                    y_start.push(*value)
                }
            },
            Self::Extended => {
                for _ in 0..number_of_end_insertions {
                    y_start.push(y[0]);
                }
            }
        }

        y_start
    }

    /// Returns the values to insert after the end of the data
    pub fn y_end_values(&self, y: &[T], number_of_end_insertions: usize) -> Vec<T> {
        let mut y_end: Vec<T> = Vec::with_capacity(number_of_end_insertions);

        match self {
            Self::Zero => {
                for _ in 0..number_of_end_insertions {
                    y_end.push(T::default());
                }
            },
            Self::Given(value) => {
                for _ in 0..number_of_end_insertions{
                    y_end.push(*value);
                }
            },
            Self::Extended => {
                let last_index = y.len() - 1;

                for _ in 0..number_of_end_insertions {
                    y_end.push(y[last_index]);
                }
            }
        }

        y_end
    }

    /// Returns the x-values to insert before the start of the data, with a spacing equal to the
    /// spacing of the first two points multiplied by `delta_x_factor`
    pub fn x_start_values(x: &[Float], number_of_end_insertions: usize, delta_x_factor: Float) -> Vec<Float> {
        let mut x_start: Vec<Float> = Vec::with_capacity(number_of_end_insertions);

        let delta_x = (x[0] - x[1]) * delta_x_factor;

        for i in (0..number_of_end_insertions).rev() {
            x_start.push(x[0] + ((i+1) as Float) * delta_x);
        }

        x_start
    }

    /// Returns the x-values to insert after the end of the data, with a spacing equal to the
    /// spacing of the last two points multiplied by `delta_x_factor`
    pub fn x_end_values(x: &[Float], number_of_end_insertions: usize, delta_x_factor: Float) -> Vec<Float> {
        let mut x_end: Vec<Float> = Vec::with_capacity(number_of_end_insertions);

        let nr_points = x.len();

        let delta_x = (x[nr_points-1] - x[nr_points-2]) * delta_x_factor;

        for i in 0..number_of_end_insertions {
            x_end.push(x[nr_points-1] + ((i+1) as Float) * delta_x);
        }

        x_end
    }

    /// Returns the x-data extended with inserted values at both ends
    pub fn add_end_values_to_x_data(
        x: &[Float], 
        number_of_end_insertions: usize, 
        delta_x_factor: Float
    ) -> Vec<Float> {

        let x_start = Self::x_start_values(x, number_of_end_insertions, delta_x_factor);
        let x_end = Self::x_end_values(x, number_of_end_insertions, delta_x_factor);

        let mut x_modified: Vec<Float> = Vec::with_capacity(x.len() + number_of_end_insertions * 2);

        x_modified.extend_from_slice(&x_start);
        x_modified.extend_from_slice(x);
        x_modified.extend_from_slice(&x_end);

        x_modified
    }

    /// Returns the y-data extended with inserted values at both ends, based on the end conditions
    /// for the start and the end
    pub fn add_end_values_to_y_data(
        y: &[T], 
        number_of_end_insertions: usize, 
        end_conditions: [Self; 2]
    ) -> Vec<T> {
        let y_start = end_conditions[0].y_start_values(y, number_of_end_insertions);
        let y_end = end_conditions[1].y_end_values(y, number_of_end_insertions);

        let mut y_modified: Vec<T> = Vec::with_capacity(y.len() + number_of_end_insertions * 2);

        y_modified.extend_from_slice(&y_start);
        y_modified.extend_from_slice(y);
        y_modified.extend_from_slice(&y_end);

        y_modified
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smoothing_end_conditions() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![1.0, 2.0, 3.0, 2.0, 1.0];

        println!("ZeroValues");
        let end_conditions = [EndCondition::Zero, EndCondition::Zero];
        let number_of_end_insertions = 3;

        let delta_x_factor = 0.5;

        let x_modified = EndCondition::<Float>::add_end_values_to_x_data(
            &x, 
            number_of_end_insertions, 
            delta_x_factor
        );
        let y_modified = EndCondition::add_end_values_to_y_data(
            &y, 
            number_of_end_insertions, 
            end_conditions
        );

        dbg!(&x_modified);
        dbg!(&y_modified);

        let x_result = vec![-0.5, 0.0, 0.5, 1.0, 2.0, 3.0, 4.0, 5.0, 5.5, 6.0, 6.5];
        let y_result = vec![0.0, 0.0, 0.0, 1.0, 2.0, 3.0, 2.0, 1.0, 0.0, 0.0, 0.0];

        assert_eq!(y_modified, y_result);
        assert_eq!(x_modified, x_result);
    }

    #[test]
    fn test_from_str() {
        assert!(matches!("Zero".parse::<EndCondition<Float>>(), Ok(EndCondition::Zero)));
        assert!(matches!("Extended".parse::<EndCondition<Float>>(), Ok(EndCondition::Extended)));
        assert!("Unknown".parse::<EndCondition<Float>>().is_err());

        assert!(matches!("Five".parse(), Ok(super::super::polynomial::WindowSize::Five)));
        assert!("Four".parse::<super::super::polynomial::WindowSize>().is_err());
    }
}