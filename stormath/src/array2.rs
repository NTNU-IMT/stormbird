// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! 2D array implementation

use std::ops::{Index, IndexMut};
use std::fmt::Debug;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Array2Indices {
    pub shape: [usize; 2],
}

impl Array2Indices {
    #[inline(always)]
    pub fn flat_index(&self, indices: [usize; 2]) -> usize {
        indices[0] * self.shape[1] + indices[1]
    }

    #[inline(always)]
    pub fn indices_from_index(&self, flat_index: usize) -> [usize; 2] {
        [flat_index / self.shape[1], flat_index % self.shape[1]]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Array2<T> {
    pub data: Vec<T>,
    pub indices: Array2Indices,
}

impl<T> Array2<T> 
where T: Default + Clone + Copy + Debug,
{
    pub fn new_default(shape: [usize; 2]) -> Self {
        let data = vec![T::default(); shape[0] * shape[1]];
        
        Self {
            data, 
            indices: Array2Indices {
                shape
            }
        }
    }

    pub fn from_elem(shape: [usize; 2], elem: T) -> Self {
        let data = vec![elem; shape[0] * shape[1]];
        
        Self {
            data, 
            indices: Array2Indices {
                shape
            }
        }
    }

    #[inline(always)]
    pub fn flat_index(&self, indices: [usize; 2]) -> usize {
        self.indices.flat_index(indices)
    }

    #[inline(always)]
    pub fn indices_from_index(&self, flat_index: usize) -> [usize; 2] {
        self.indices.indices_from_index(flat_index)
    }

    pub fn nr_rows(&self) -> usize {
        self.indices.shape[0]
    }

    pub fn nr_cols(&self) -> usize {
        self.indices.shape[1]
    }

    pub fn shape(&self) -> [usize; 2] {
        self.indices.shape
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

impl<T> std::ops::Add for Array2<T> 
where T: Default + Clone + Copy + Debug + std::ops::Add<Output = T>,
{
    type Output = Self;

    fn add(self, other: Self) -> Self {
        assert_eq!(self.shape(), other.shape(), "Array2 shapes do not match");
        
        let mut result = self.clone();
        
        for i in 0..self.data.len() {
            result.data[i] = self.data[i] + other.data[i];
        }
        
        result
    }
}