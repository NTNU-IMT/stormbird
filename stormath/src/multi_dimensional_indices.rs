
#[derive(Debug, Clone)]
/// Structure used to manage data that is stored in a two-dimensional array.
/// 
/// The primary purpose is to be able to convert to and from dimensional and flat indices.
pub struct TwoDimensionalIndices {
    pub nr_rows: usize,
    pub nr_cols: usize,
}

impl TwoDimensionalIndices {
    #[inline(always)]
    pub fn total_nr_elements(&self) -> usize {
        self.nr_rows * self.nr_cols
    }

    #[inline(always)]
    pub fn flat_index(&self, indices: &[usize; 2]) -> usize {
        indices[0] * self.nr_cols + indices[1]
    }

    #[inline(always)]
    pub fn dimensional_index(&self, flat_index: usize) -> [usize; 2] {
        [flat_index / self.nr_cols, flat_index % self.nr_cols]
    }
}