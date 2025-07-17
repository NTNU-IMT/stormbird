use super::*;

impl<T> Matrix<T> 
where T: Default + Clone + Copy + Debug,
{
    /// Transposes the matrix, swapping rows and columns.
    pub fn transpose(&self) -> Self {
        let mut result = Matrix::new_default([self.shape[1], self.shape[0]]);

        for i in 0..self.shape[0] {
            for j in 0..self.shape[1] {
                result[[j, i]] = self[[i, j]];
            }
        }

        result
    }
}
