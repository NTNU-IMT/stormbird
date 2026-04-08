
use stormath::type_aliases::Float;

use std::ops::{Index, IndexMut, Add, Sub, Mul};

#[derive(Debug, Clone)]
/// Structure for storing vector fields where the values are staggered, and where access to 
/// individual components typically happens without the need to access other components at the same
/// time. The internal data is therefore stored as continuous vectors for each component, rather than
/// vectors of x, y, and z values stored together
pub struct StaggeredSpatialVectors {
    pub data: [Vec<Float>; 3]
}

impl StaggeredSpatialVectors {
    pub fn new_default(length: usize) -> Self {
        Self {
            data: [
                vec![0.0; length],
                vec![0.0; length],
                vec![0.0; length]
            ]
        }
    }
}

impl Index<[usize; 2]> for StaggeredSpatialVectors {
    type Output = Float;

    fn index(&self, indices: [usize; 2]) -> &Self::Output {
        let axis_index = indices[0];
        let value_index = indices[1];
        
        &self.data[axis_index][value_index]
    }
}

impl IndexMut<[usize; 2]> for StaggeredSpatialVectors {
    fn index_mut(&mut self, indices: [usize; 2]) -> &mut Self::Output {
        let axis_index = indices[0];
        let value_index = indices[1];
        
        &mut self.data[axis_index][value_index]
    }
}

impl Add for StaggeredSpatialVectors {
    type Output = Self;
    
    fn add(self, other: Self) -> Self {
        assert_eq!(self.data[0].len(), other.data[0].len(), "Length does not match");
        
        let mut result = self.clone();
        
        for axis_index in 0..3 {
            for data_index in 0..self.data[axis_index].len() {
                result.data[axis_index][data_index] += other.data[axis_index][data_index];
            }
        }
        
        result
    }
}

impl Sub for StaggeredSpatialVectors {
    type Output = Self;
    
    fn sub(self, other: Self) -> Self {
        assert_eq!(self.data[0].len(), other.data[0].len(), "Length does not match");
        
        let mut result = self.clone();
        
        for axis_index in 0..3 {
            for data_index in 0..self.data[axis_index].len() {
                result.data[axis_index][data_index] -= other.data[axis_index][data_index];
            }
        }
        
        result
    }
}

impl Mul<Float> for StaggeredSpatialVectors {
    type Output = Self;

    fn mul(self, scalar: Float) -> Self {
        let mut result = self.clone();

        for axis_index in 0..3 {
            for data_index in 0..self.data[axis_index].len() {
                result.data[axis_index][data_index] *= scalar;
            }
        }

        result
    }
}

impl Mul<StaggeredSpatialVectors> for Float {
    type Output = StaggeredSpatialVectors;

    fn mul(self, vectors: StaggeredSpatialVectors) -> StaggeredSpatialVectors {
        let mut result = vectors.clone();
        
        for axis_index in 0..3 {
            for data_index in 0..result.data[axis_index].len() {
                result.data[axis_index][data_index] *= self;
            }
        }
        
        result
    }
}
