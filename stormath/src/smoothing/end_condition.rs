
use super::*;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
/// Enum used to decide how to modify input data used in smoothing functions. 
pub enum EndCondition {
    /// Pads the ends with zeros
    ZeroValues,
    /// Mirrors the values at the ends
    MirroredValues,
    /// Mirrors the values with an opposite sign
    ReversedMirroredValues,
    /// Pads the ends with the same value as the end values
    ExtendedValues
}

impl EndCondition {
    pub fn from_str(end_condition: &str) -> Self {
        match end_condition {
            "ZeroValues" => Self::ZeroValues,
            "MirroredValues" => Self::MirroredValues,
            "ReversedMirroredValues" => Self::ReversedMirroredValues,
            "ExtendedValues" => Self::ExtendedValues,
            _ => panic!("Unknown smoothing end condition: {}", end_condition)
        }
    }

    pub fn y_start_values<T>(&self, y: &[T], number_of_end_insertions: usize) -> Vec<T> 
    where T: SmoothingOps
    {
        let mut y_start: Vec<T> = Vec::with_capacity(number_of_end_insertions + 1);

        match self {
            Self::ZeroValues => {
                for _ in 0..number_of_end_insertions + 1 {
                    y_start.push(T::default());
                }
            }
            Self::MirroredValues => {
                for i in 0..number_of_end_insertions {
                    y_start.push(y[number_of_end_insertions - i]);
                }
            }
            Self::ReversedMirroredValues => {
                y_start.push(T::default());

                for i in 0..number_of_end_insertions {
                    y_start.push(-y[number_of_end_insertions - i]);
                }
            }
            Self::ExtendedValues => {
                for _ in 0..number_of_end_insertions {
                    y_start.push(y[0]);
                }
            }
        }

        y_start
    }

    pub fn y_end_values<T>(&self, y: &[T], number_of_end_insertions: usize) -> Vec<T> 
    where T: SmoothingOps
    {
        let mut y_end: Vec<T> = Vec::with_capacity(number_of_end_insertions);

        match self {
            Self::ZeroValues => {
                for _ in 0..number_of_end_insertions + 1 {
                    y_end.push(T::default());
                }
            }
            Self::MirroredValues => {
                let last_index = y.len() - 1;

                for i in 0..number_of_end_insertions {
                    y_end.push(y[last_index - i - 1]);
                }
            }
            Self::ReversedMirroredValues => {
                let last_index = y.len() - 1;

                y_end.push(T::default());

                for i in 0..number_of_end_insertions {
                    y_end.push(-y[last_index - i - 1]);
                }
            },
            Self::ExtendedValues => {
                let last_index = y.len() - 1;

                for _ in 0..number_of_end_insertions {
                    y_end.push(y[last_index]);
                }
            }
        }

        y_end
    }

    pub fn x_start_values(self, x: &[f64], number_of_end_insertions: usize) -> Vec<f64> {
        let mut x_start: Vec<f64> = Vec::with_capacity(number_of_end_insertions + 1);

        let delta_x = x[0] - x[1];
        
        match self {
            Self::ZeroValues | Self::ReversedMirroredValues => {
                for i in (0..number_of_end_insertions).rev() {
                    x_start.push(x[0] + ((i+1) as f64) * delta_x);
                }

                x_start.push(x[0] + 0.5 * delta_x);
            },
            _ => {
                for i in (0..number_of_end_insertions).rev() {
                    x_start.push(x[0] + ((i+1) as f64) * delta_x);
                }
            }
        }

        x_start
    }

    pub fn x_end_values(self, x: &[f64], number_of_end_insertions: usize) -> Vec<f64> {
        let mut x_end: Vec<f64> = Vec::with_capacity(number_of_end_insertions + 1);

        let nr_points = x.len();

        let delta_x = x[nr_points-1] - x[nr_points-2];
        
        match self {
            Self::ZeroValues | Self::ReversedMirroredValues => {
                x_end.push(x[nr_points-1] + 0.5 * delta_x);

                for i in 0..number_of_end_insertions {
                    x_end.push(x[nr_points-1] + ((i+1) as f64) * delta_x);
                }
            },
            _ => {
                for i in 0..number_of_end_insertions {
                    x_end.push(x[nr_points-1] + ((i+1) as f64) * delta_x);
                }
            }
        }

        x_end
    }

    pub fn add_end_values_to_x_data(
        x: &[f64], 
        number_of_end_insertions: usize, 
        end_conditions: [Self; 2]
    ) -> Vec<f64> {

        let x_start = end_conditions[0].x_start_values(x, number_of_end_insertions);
        let x_end = end_conditions[1].x_end_values(x, number_of_end_insertions);

        let mut x_modified: Vec<f64> = Vec::with_capacity(x.len() + number_of_end_insertions * 2);

        x_modified.extend_from_slice(&x_start);
        x_modified.extend_from_slice(x);
        x_modified.extend_from_slice(&x_end);

        x_modified
    }

    pub fn add_end_values_to_y_data<T>(
        y: &[T], 
        number_of_end_insertions: usize, 
        end_conditions: [Self; 2]
    ) -> Vec<T> 
    where T: SmoothingOps
    {
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
        let end_conditions = [EndCondition::ZeroValues, EndCondition::ZeroValues];
        let number_of_end_insertions = 2;

        let x_modified = EndCondition::add_end_values_to_x_data(&x, number_of_end_insertions, end_conditions);
        let y_modified = EndCondition::add_end_values_to_y_data(&y, number_of_end_insertions, end_conditions);

        dbg!(&x_modified);
        dbg!(&y_modified);

        let x_result = vec![-1.0, 0.0, 0.5, 1.0, 2.0, 3.0, 4.0, 5.0, 5.5, 6.0, 7.0];
        let y_result = vec![0.0, 0.0, 0.0, 1.0, 2.0, 3.0, 2.0, 1.0, 0.0, 0.0, 0.0];

        assert_eq!(y_modified, y_result);
        assert_eq!(x_modified, x_result);
    }
}