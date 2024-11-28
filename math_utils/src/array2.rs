use std::ops::{Index, IndexMut};
use std::fmt::Debug;

#[derive(Debug, Clone)]
pub struct Array2<T> {
    pub data: Vec<T>,
    pub shape: [usize; 2],
}

impl<T> Array2<T> 
where T: Default + Clone + Copy + Debug,
{
    pub fn new_default(shape: [usize; 2]) -> Self {
        let data = vec![T::default(); shape[0] * shape[1]];
        
        Self {
            data, 
            shape
        }
    }

    pub fn from_elem(shape: [usize; 2], elem: T) -> Self {
        let data = vec![elem; shape[0] * shape[1]];
        
        Self {
            data, 
            shape
        }
    }

    #[inline(always)]
    pub fn flat_index(&self, indices: [usize; 2]) -> usize {
        indices[0] * self.shape[1] + indices[1]
    }

    #[inline(always)]
    pub fn indices_from_index(&self, flat_index: usize) -> [usize; 2] {
        [flat_index / self.shape[1], flat_index % self.shape[1]]
    }

    pub fn shape(&self) -> [usize; 2] {
        self.shape
    }
}

impl<T> Index<[usize; 2]> for Array2<T> 
where T: Default + Clone + Copy + Debug,
{
    type Output = T;

    fn index(&self, indices: [usize; 2]) -> &Self::Output {
        let flat_index = self.flat_index(indices);
        &self.data[flat_index]
    }
}

impl<T> IndexMut<[usize; 2]> for Array2<T> 
where T: Default + Clone + Copy + Debug,
{
    fn index_mut(&mut self, indices: [usize; 2]) -> &mut Self::Output {
        let flat_index = self.flat_index(indices);
        
        &mut self.data[flat_index]
    }
}