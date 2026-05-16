
use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;
use stormath::sparse_matrix::SparseMatrix;

pub const INTERIOR_OFFSET: usize = 1;

#[derive(Debug, Clone)]
/// Structure for storing indices around a cell/face
pub struct LocalFlatIndices {
    pub current: usize,
    pub pos: [usize; 3],
    pub neg: [usize; 3],
    /// Edge indices: pos_neg[i][j] is the index shifted +1 in direction i and -1 in direction j.
    /// For i == j, these values are undefined (set to current as a placeholder, but should not be used).
    pub pos_neg: [[usize; 3]; 3],
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
        let [i, j, k] = indices;
        
        LocalFlatIndices {
            current: self.flat_index_on_extended_grid(indices),
            pos: [
                self.flat_index_on_extended_grid([i+1, j, k]),
                self.flat_index_on_extended_grid([i, j+1, k]),
                self.flat_index_on_extended_grid([i, j, k+1])
            ],
            neg: [
                self.flat_index_on_extended_grid([i-1, j, k]),
                self.flat_index_on_extended_grid([i, j-1, k]),
                self.flat_index_on_extended_grid([i, j, k-1])
            ],
            pos_neg: [
                [   // a = 0: +1 in x-direction
                    self.flat_index_on_extended_grid([i, j, k]),     // d=0: placeholder (a==d)
                    self.flat_index_on_extended_grid([i+1, j-1, k]), // d=1: +x, -y
                    self.flat_index_on_extended_grid([i+1, j, k-1]), // d=2: +x, -z
                ],
                [   // a = 1: +1 in y-direction
                    self.flat_index_on_extended_grid([i-1, j+1, k]), // d=0: +y, -x
                    self.flat_index_on_extended_grid([i, j, k]),     // d=1: placeholder (a==d)
                    self.flat_index_on_extended_grid([i, j+1, k-1]), // d=2: +y, -z
                ],
                [   // a = 2: +1 in z-direction
                    self.flat_index_on_extended_grid([i-1, j, k+1]), // d=0: +z, -x
                    self.flat_index_on_extended_grid([i, j-1, k+1]), // d=1: +z, -y
                    self.flat_index_on_extended_grid([i, j, k]),     // d=2: placeholder (a==d)
                ],
            ],
        }
    }
    
    pub fn local_flat_indices_on_interior_grid(&self, indices: [usize; 3]) -> LocalFlatIndices {
        let [nx, ny, nz] = self.nr_interior_cells();
        
        let [i, j, k] = indices;
        
        LocalFlatIndices {
            current: self.flat_index_on_interior_grid(indices),
            pos: [
                self.flat_index_on_interior_grid([i.saturating_add(1).min(nx-1), j, k]),
                self.flat_index_on_interior_grid([i, j.saturating_add(1).min(ny-1), k]),
                self.flat_index_on_interior_grid([i, j, k.saturating_add(1).min(nz-1)])
            ],
            neg: [
                self.flat_index_on_interior_grid([i.saturating_sub(1), j, k]),
                self.flat_index_on_interior_grid([i, j.saturating_sub(1), k]),
                self.flat_index_on_interior_grid([i, j, k.saturating_sub(1)])
            ],
            pos_neg: [
                [   // a = 0: +1 in x-direction
                    self.flat_index_on_extended_grid([i, j, k]),     // d=0: placeholder (a==d)
                    self.flat_index_on_extended_grid([i+1, j.saturating_sub(1), k]), // d=1: +x, -y
                    self.flat_index_on_extended_grid([i+1, j, k.saturating_sub(1)]), // d=2: +x, -z
                ],
                [   // a = 1: +1 in y-direction
                    self.flat_index_on_extended_grid([i.saturating_sub(1), j+1, k]), // d=0: +y, -x
                    self.flat_index_on_extended_grid([i, j, k]),     // d=1: placeholder (a==d)
                    self.flat_index_on_extended_grid([i, j+1, k.saturating_sub(1)]), // d=2: +y, -z
                ],
                [   // a = 2: +1 in z-direction
                    self.flat_index_on_extended_grid([i.saturating_sub(1), j, k+1]), // d=0: +z, -x
                    self.flat_index_on_extended_grid([i, j.saturating_sub(1), k+1]), // d=1: +z, -y
                    self.flat_index_on_extended_grid([i, j, k]),     // d=2: placeholder (a==d)
                ],
            ],
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
    
    /// Creates a new grid that is coarser than `self` by a factor of 2 in each dimension.
    /// 
    /// The coarse grid has exactly half the number of interior cells in each dimension,
    /// with cell lengths doubled. The domain start point and overall extent remain the same.
    /// 
    /// # Panics
    /// Panics if any dimension has an odd number of interior cells.
    pub fn coarsened(&self) -> Grid {
        let [nx, ny, nz] = self.nr_interior_cells();
        
        assert!(
            nx % 2 == 0 && ny % 2 == 0 && nz % 2 == 0,
            "Cannot coarsen grid: interior cell counts must be even. Got [{}, {}, {}]",
            nx, ny, nz
        );
        
        Grid {
            start_point: self.start_point,
            cell_length: SpatialVector([
                self.cell_length[0] * 2.0,
                self.cell_length[1] * 2.0,
                self.cell_length[2] * 2.0,
            ]),
            nr_extended_cells: [
                nx / 2 + 2 * INTERIOR_OFFSET,
                ny / 2 + 2 * INTERIOR_OFFSET,
                nz / 2 + 2 * INTERIOR_OFFSET,
            ],
        }
    }
    
    /// Computes the restriction operator as a sparse matrix for transferring cell-centered
    /// interior values from this (fine) grid to a coarser grid.
    /// 
    /// The coarse grid has exactly half the number of interior cells in each dimension.
    /// Each coarse cell value is computed as the average of the 8 fine cells it contains
    /// (full-weighting restriction).
    /// 
    /// Matrix shape: [coarse_size, fine_size] where each row has 8 entries with weight 1/8.
    pub fn restriction_operator<const N: usize>(&self) -> SparseMatrix<N> {
        let [nx_f, ny_f, nz_f] = self.nr_interior_cells();
        
        // Coarse grid has half the cells in each dimension
        let nx_c = nx_f / 2;
        let ny_c = ny_f / 2;
        let nz_c = nz_f / 2;
        
        let fine_size = nx_f * ny_f * nz_f;
        let coarse_size = nx_c * ny_c * nz_c;
        
        let mut matrix: SparseMatrix<N> = SparseMatrix::new_default(coarse_size, fine_size);
        
        let weight = 1.0 / 8.0;
        
        // For each coarse cell, average the 8 fine cells it contains
        for i_c in 0..nx_c {
            for j_c in 0..ny_c {
                for k_c in 0..nz_c {
                    // Coarse cell row index
                    let coarse_row = i_c * ny_c * nz_c + j_c * nz_c + k_c;
                    
                    // The 8 fine cells that this coarse cell covers
                    for di in 0..2 {
                        for dj in 0..2 {
                            for dk in 0..2 {
                                let i_f = 2 * i_c + di;
                                let j_f = 2 * j_c + dj;
                                let k_f = 2 * k_c + dk;
                                
                                let fine_col = self.flat_index_on_interior_grid([i_f, j_f, k_f]);
                                
                                matrix[[coarse_row, fine_col]] = weight;
                            }
                        }
                    }
                }
            }
        }
        
        matrix
    }
    
    /// Computes the prolongation operator as a sparse matrix for transferring cell-centered
    /// interior values from a coarser grid to this (fine) grid.
    /// 
    /// The coarse grid has exactly half the number of interior cells in each dimension.
    /// Uses trilinear interpolation from the coarse cell centers to the fine cell centers.
    /// 
    /// Matrix shape: [fine_size, coarse_size] where each row has up to 8 entries.
    pub fn prolongation_operator<const N: usize>(&self) -> SparseMatrix<N> {
        let [nx_f, ny_f, nz_f] = self.nr_interior_cells();
        
        // Coarse grid has half the cells in each dimension
        let nx_c = nx_f / 2;
        let ny_c = ny_f / 2;
        let nz_c = nz_f / 2;
        
        let fine_size = nx_f * ny_f * nz_f;
        let coarse_size = nx_c * ny_c * nz_c;
        
        let mut matrix: SparseMatrix<N> = SparseMatrix::new_default(fine_size, coarse_size);
        
        // For each fine cell, compute trilinear interpolation weights from coarse cells
        for i_f in 0..nx_f {
            for j_f in 0..ny_f {
                for k_f in 0..nz_f {
                    let fine_row = self.flat_index_on_interior_grid([i_f, j_f, k_f]);
                    
                    // Fine cell center position in "coarse cell units"
                    // Fine cell i_f has center at (i_f + 0.5) * dx_f = (i_f + 0.5) * dx_c / 2
                    // In coarse cell units (where coarse cell j has center at j + 0.5),
                    // the fine cell center is at: (i_f + 0.5) / 2 = i_f/2 + 0.25
                    // We want position relative to coarse cell centers at j + 0.5,
                    // so xi = (i_f + 0.5) / 2 - 0.5 = i_f/2 - 0.25
                    let xi = (i_f as Float) / 2.0 - 0.25;
                    let eta = (j_f as Float) / 2.0 - 0.25;
                    let zeta = (k_f as Float) / 2.0 - 0.25;
                    
                    // Find the "lower" coarse cell index for interpolation
                    // The coarse cell with center at i_c + 0.5 <= xi < i_c + 1.5
                    // means i_c = floor(xi - 0.5) but we clamp to valid range
                    let i_c_base = (xi.floor() as isize).max(0).min((nx_c - 1) as isize) as usize;
                    let j_c_base = (eta.floor() as isize).max(0).min((ny_c - 1) as isize) as usize;
                    let k_c_base = (zeta.floor() as isize).max(0).min((nz_c - 1) as isize) as usize;
                    
                    // Local coordinates within the interpolation stencil [0, 1]
                    let mut sx = xi - (i_c_base as Float);
                    let mut sy = eta - (j_c_base as Float);
                    let mut sz = zeta - (k_c_base as Float);
                    
                    // Clamp to [0, 1] and determine which coarse cells contribute
                    sx = sx.clamp(0.0, 1.0);
                    sy = sy.clamp(0.0, 1.0);
                    sz = sz.clamp(0.0, 1.0);
                    
                    // Trilinear interpolation weights
                    let wx = [1.0 - sx, sx];
                    let wy = [1.0 - sy, sy];
                    let wz = [1.0 - sz, sz];
                    
                    for di in 0..2 {
                        for dj in 0..2 {
                            for dk in 0..2 {
                                let i_c = (i_c_base + di).min(nx_c - 1);
                                let j_c = (j_c_base + dj).min(ny_c - 1);
                                let k_c = (k_c_base + dk).min(nz_c - 1);
                                
                                let weight = wx[di] * wy[dj] * wz[dk];
                                
                                if weight > 0.0 {
                                    let coarse_col = i_c * ny_c * nz_c + j_c * nz_c + k_c;
                                    
                                    // Add to existing weight if this coarse cell was already added
                                    // (can happen at boundaries due to clamping)
                                    matrix[[fine_row, coarse_col]] += weight;
                                }
                            }
                        }
                    }
                }
            }
        }
        
        matrix
    }
}
