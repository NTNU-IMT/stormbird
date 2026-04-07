
use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

pub const INTERIOR_OFFSET: usize = 1;

#[derive(Debug, Clone)]
/// Structure for storing indices around a cell/face
pub struct LocalFlatIndices {
    pub current: usize,
    pub x_neg: [usize; 1],
    pub x_pos: [usize; 1],
    pub y_neg: [usize; 1],
    pub y_pos: [usize; 1],
    pub z_neg: [usize; 1],
    pub z_pos: [usize; 1]
}

#[derive(Debug, Clone)]
/// Structured grid definition
pub struct Grid {
    pub start_point: SpatialVector,
    pub cell_length: SpatialVector,
    pub nr_extended_cells: [usize; 3],
}

impl Grid {
    #[inline(always)]
    pub fn nr_interior_cells(&self) -> [usize; 3] {
        [
            self.nr_extended_cells[0] - 2 * INTERIOR_OFFSET,
            self.nr_extended_cells[1] - 2 * INTERIOR_OFFSET,
            self.nr_extended_cells[2] - 2 * INTERIOR_OFFSET
        ]
    }
    
    /// Returns the index to values that exist on the full extended grid, from the indices in x, y 
    /// and z direction respectively.
    pub fn flat_index_on_extended_grid(&self, indices: [usize; 3]) -> usize {
        indices[0] * self.nr_extended_cells[1] * self.nr_extended_cells[2] +
        indices[1] * self.nr_extended_cells[2] + 
        indices[2]
    }
    
    /// Returns the index to values that exist on the interior grid, from the interior indices in x, 
    /// y and z direction respectively.
    pub fn flat_index_on_interior_grid(&self, indices: [usize; 3]) -> usize {
        let nr_interior_cells = self.nr_interior_cells();
        
        indices[0] * nr_interior_cells[1] * nr_interior_cells[2] +
        indices[1] * nr_interior_cells[2] + 
        indices[2]
    }
    
    pub fn extended_indices_from_interior_indices(&self, interior_indices: [usize; 3]) -> [usize; 3] {
        [
            interior_indices[0] + INTERIOR_OFFSET,
            interior_indices[1] + INTERIOR_OFFSET,
            interior_indices[2] + INTERIOR_OFFSET,
        ]
    }
    
    pub fn flat_index_on_extended_grid_from_interior_indices(&self, interior_indices: [usize; 3]) -> usize {
        let extended_indices = self.extended_indices_from_interior_indices(interior_indices);
        
        self.flat_index_on_extended_grid(extended_indices)
    }
    
    pub fn local_flat_indices_on_extended_grid(&self, indices: [usize; 3]) -> LocalFlatIndices {
        LocalFlatIndices {
            current: self.flat_index_on_extended_grid(indices),
            x_neg: [self.flat_index_on_extended_grid([indices[0]-1, indices[1], indices[2]])],
            x_pos: [self.flat_index_on_extended_grid([indices[0]+1, indices[1], indices[2]])],
            y_neg: [self.flat_index_on_extended_grid([indices[0], indices[1]-1, indices[2]])],
            y_pos: [self.flat_index_on_extended_grid([indices[0], indices[1]+1, indices[2]])],
            z_neg: [self.flat_index_on_extended_grid([indices[0], indices[1], indices[2]-1])],
            z_pos: [self.flat_index_on_extended_grid([indices[0], indices[1], indices[2]+1])],
        }
    }
    
    pub fn local_flat_indices_on_interior_grid(&self, indices: [usize; 3]) -> LocalFlatIndices {
        let [nx, ny, nz] = self.nr_interior_cells();
        
        LocalFlatIndices {
            current: self.flat_index_on_interior_grid(indices),
            x_neg: [self.flat_index_on_interior_grid([indices[0].saturating_sub(1), indices[1], indices[2]])],
            x_pos: [self.flat_index_on_interior_grid([indices[0].saturating_add(1).min(nx-1), indices[1], indices[2]])],
            y_neg: [self.flat_index_on_interior_grid([indices[0], indices[1].saturating_sub(1), indices[2]])],
            y_pos: [self.flat_index_on_interior_grid([indices[0], indices[1].saturating_add(1).min(ny-1), indices[2]])],
            z_neg: [self.flat_index_on_interior_grid([indices[0], indices[1], indices[2].saturating_sub(1)])],
            z_pos: [self.flat_index_on_interior_grid([indices[0], indices[1], indices[2].saturating_add(1).min(nz-1)])],
        }
    }
    
    pub fn interior_indices_from_flat_index(&self, flat_index: usize) -> [usize; 3] {
        let nr_interior_cells = self.nr_interior_cells();
        
        let nynz = nr_interior_cells[1] * nr_interior_cells[2];
        let ix = flat_index / nynz;
        let iy = (flat_index % nynz) / nr_interior_cells[2];
        let iz = flat_index % nr_interior_cells[2];
        
        [ix, iy, iz]
    }

    pub fn extended_indices_from_flat_index(&self, flat_index: usize) -> [usize; 3] {
        let nynz = self.nr_extended_cells[1] * self.nr_extended_cells[2];
        let ix = flat_index / nynz;
        let iy = (flat_index % nynz) / self.nr_extended_cells[2];
        let iz = flat_index % self.nr_extended_cells[2];
        
        [ix, iy, iz]
    }
    
    pub fn transfer_interior_values_to_extended_grid(
        &self, 
        interior_values: &[Float], 
        extended_values: &mut [Float]
    ) {
        let [nx, ny, nz] = self.nr_interior_cells();
        
        for i_x in 0..nx {
            for i_y in 0..ny {
                for i_z in 0..nz {
                    let interior_indices = [i_x, i_y, i_z];
                    let flat_index_interior = self.flat_index_on_interior_grid(interior_indices);
                    
                    let extended_indices = self.extended_indices_from_interior_indices(interior_indices);
                    let flat_index_extended = self.flat_index_on_extended_grid(extended_indices);
                    
                    extended_values[flat_index_extended] = interior_values[flat_index_interior];
                }
            }
        }
    }
    
    pub fn interior_values_from_extended_values(
        &self,
        extended_values: &[Float], 
    ) -> Vec<Float> {
        let [nx, ny, nz] = self.nr_interior_cells();
        
        let mut out = vec![0.0; nx*ny*nz];
        
        for i_x in 0..nx {
            for i_y in 0..ny {
                for i_z in 0..nz {
                    let interior_indices = [i_x, i_y, i_z];
                    let flat_index_interior = self.flat_index_on_interior_grid(interior_indices);
                    
                    let extended_indices = self.extended_indices_from_interior_indices(interior_indices);
                    let flat_index_extended = self.flat_index_on_extended_grid(extended_indices);
                    
                    out[flat_index_interior] = extended_values[flat_index_extended];
                }
            }
        }
        
        out
    }
    
    /// Returns the coordinate of the cell center from the indices given. 
    pub fn cell_center(&self, interior_indices: [usize; 3]) -> SpatialVector {
        let start_cell_center = self.start_point + 0.5 * self.cell_length;
        
        SpatialVector(
            [
                start_cell_center[0] + (interior_indices[0] as Float) * self.cell_length[0],
                start_cell_center[1] + (interior_indices[1] as Float) * self.cell_length[1],
                start_cell_center[2] + (interior_indices[2] as Float) * self.cell_length[2],
            ]
        )
    }
}