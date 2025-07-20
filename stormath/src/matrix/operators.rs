use std::ops::{Index, IndexMut, Add, Sub, Mul};

use super::*;

impl<T> Index<[usize; 2]> for Matrix<T> 
where T: Default + Clone + Copy + Debug,
{
    type Output = T;

    fn index(&self, indices: [usize; 2]) -> &Self::Output {
        let flat_index = self.flat_index(indices);
        &self.data[flat_index]
    }
}

impl<T> IndexMut<[usize; 2]> for Matrix<T> 
where T: Default + Clone + Copy + Debug,
{
    fn index_mut(&mut self, indices: [usize; 2]) -> &mut Self::Output {
        let flat_index = self.flat_index(indices);
        
        &mut self.data[flat_index]
    }
}

impl<T> Add for Matrix<T> 
where T: Default + Clone + Copy + Debug + Add<Output = T>,
{
    type Output = Self;

    fn add(self, other: Self) -> Self {
        assert_eq!(self.shape, other.shape, "Shapes do not match");
        
        let mut result = self.clone();
        
        for i in 0..self.data.len() {
            result.data[i] = self.data[i] + other.data[i];
        }
        
        result
    }
}

impl<T> Sub for Matrix<T> 
where T: Default + Clone + Copy + Debug + Sub<Output = T>,
{
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        assert_eq!(self.shape, other.shape, "Shapes do not match");
        
        let mut result = self.clone();
        
        for i in 0..self.data.len() {
            result.data[i] = self.data[i] - other.data[i];
        }
        
        result
    }
}

impl<T> Mul<f64> for Matrix<T> 
where T: Default + Clone + Copy + Debug + Mul<f64, Output = T>,
{
    type Output = Self;

    fn mul(self, scalar: f64) -> Self {
        let mut result = self.clone();
        
        for i in 0..self.data.len() {
            result.data[i] = self.data[i] * scalar;
        }
        
        result
    }
}

impl<T> Mul<Matrix<T>> for f64 
where T: Default + Clone + Copy + Debug + Mul<f64, Output = T>,
{
    type Output = Matrix<T>;

    fn mul(self, matrix: Matrix<T>) -> Matrix<T> {
        let mut result = matrix.clone();
        
        for i in 0..matrix.data.len() {
            result.data[i] = matrix.data[i] * self;
        }
        
        result
    }
}


impl<T> Matrix<T>
where T: Default + Clone + Copy + Debug + 
    Mul<Output = T> +
    Add<Output = T>,

{
    pub fn matrix_multiply(&self, other: &Matrix<T>) -> Matrix<T> {
        assert_eq!(self.shape[1], other.shape[0], "Matrix shapes do not match for multiplication");

        let mut result = Matrix::new_default([self.shape[0], other.shape[1]]);

        for i in 0..self.shape[0] {
            for j in 0..other.shape[1] {
                let mut sum = T::default();
                for k in 0..self.shape[1] {
                    sum = sum + self[[i, k]] * other[[k, j]];
                }
                result[[i, j]] = sum;
            }
        }

        result
    }

    pub fn vector_multiply(&self, vector: &[T]) -> Vec<T> {
        assert_eq!(self.shape[1], vector.len(), "Matrix and vector shapes do not match for multiplication");

        let mut result = vec![T::default(); self.shape[0]];

        for i in 0..self.shape[0] {
            let mut sum = T::default();
            for j in 0..self.shape[1] {
                sum = sum + self[[i, j]] * vector[j];
            }
            result[i] = sum;
        }

        result
    }
}