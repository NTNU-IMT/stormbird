
use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

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
    pub extended_shape: [usize; 3],
    pub extended_stride: [usize; 2],
    pub interior_shape: [usize; 3],
    pub interior_stride: [usize; 2]
}

impl Grid {
    pub fn new(
        start_point: SpatialVector, 
        end_point: SpatialVector,
        interior_shape: [usize; 3]
    ) -> Self {
        let domain_length = end_point - start_point;
        
        let cell_length = SpatialVector([
            domain_length[0] / interior_shape[0] as Float,
            domain_length[1] / interior_shape[1] as Float,
            domain_length[2] / interior_shape[2] as Float,
        ]);
        
        let extended_shape = [
            interior_shape[0] + 2 * INTERIOR_OFFSET,
            interior_shape[1] + 2 * INTERIOR_OFFSET,
            interior_shape[2] + 2 * INTERIOR_OFFSET,
        ];

        let extended_stride = [
            extended_shape[1] * extended_shape[2], 
            extended_shape[2]
        ];

        let interior_stride = [
            interior_shape[1] * interior_shape[2],
            interior_shape[2]
        ];

        Self {
            start_point,
            cell_length,
            extended_shape,
            extended_stride,
            interior_shape,
            interior_stride
        }
    }
    
    #[inline(always)]
    pub fn nr_interior_cells(&self) -> usize {
        self.interior_shape[0] * self.interior_shape[1] * self.interior_shape[2]
    }

    #[inline(always)]
    pub fn nr_extended_cells(&self) -> usize {
        self.extended_shape[0] * self.extended_shape[1] * self.extended_shape[2]
    }

    #[inline(always)]
    /// Returns the index to values that exist on the full extended grid, from the indices in x, y 
    /// and z direction respectively.
    pub fn flat_index_on_extended_grid(&self, indices: [usize; 3]) -> usize {
        indices[0] * self.extended_stride[0] +
        indices[1] * self.extended_stride[1] + 
        indices[2]
    }

    #[inline(always)]
    /// Returns the index to values that exist on the interior grid, from the interior indices in x, 
    /// y and z direction respectively.
    pub fn flat_index_on_interior_grid(&self, indices: [usize; 3]) -> usize { 
        indices[0] * self.interior_stride[0] +
        indices[1] * self.interior_stride[1] + 
        indices[2]
    }

    #[inline(always)]
    pub fn extended_indices_from_interior_indices(&self, interior_indices: [usize; 3]) -> [usize; 3] {
        [
            interior_indices[0] + INTERIOR_OFFSET,
            interior_indices[1] + INTERIOR_OFFSET,
            interior_indices[2] + INTERIOR_OFFSET,
        ]
    }

    #[inline(always)]
    pub fn interior_indices_from_extended_indices(&self, extended_indices: [usize; 3]) -> [usize; 3] {
        [
            extended_indices[0] - INTERIOR_OFFSET,
            extended_indices[1] - INTERIOR_OFFSET,
            extended_indices[2] - INTERIOR_OFFSET,
        ]
    }

    #[inline(always)]
    pub fn flat_index_on_extended_grid_from_interior_indices(&self, interior_indices: [usize; 3]) -> usize {
        let extended_indices = self.extended_indices_from_interior_indices(interior_indices);
        
        self.flat_index_on_extended_grid(extended_indices)
    }

    #[inline(always)]
    pub fn is_cell_interior(&self, indices: [usize; 3]) -> bool {
        let s = &self.extended_shape;
        (indices[0].wrapping_sub(1) < s[0].wrapping_sub(2))
            & (indices[1].wrapping_sub(1) < s[1].wrapping_sub(2))
            & (indices[2].wrapping_sub(1) < s[2].wrapping_sub(2))
    }

    #[inline(always)]
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

    #[inline(always)]
    pub fn local_flat_indices_on_interior_grid(&self, indices: [usize; 3]) -> LocalFlatIndices {
        let [nx, ny, nz] = self.interior_shape;
        
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

    #[inline(always)]
    pub fn interior_indices_from_flat_index(&self, flat_index: usize) -> [usize; 3] {        
        let nynz = self.interior_shape[1] * self.interior_shape[2];
        let ix = flat_index / nynz;
        let iy = (flat_index % nynz) / self.interior_shape[2];
        let iz = flat_index % self.interior_shape[2];
        
        [ix, iy, iz]
    }

    #[inline(always)]
    pub fn extended_indices_from_flat_index(&self, flat_index: usize) -> [usize; 3] {
        let nynz = self.extended_shape[1] * self.extended_shape[2];
        let ix = flat_index / nynz;
        let iy = (flat_index % nynz) / self.extended_shape[2];
        let iz = flat_index % self.extended_shape[2];
        
        [ix, iy, iz]
    }
    
    pub fn transfer_interior_values_to_extended_grid(
        &self, 
        interior_values: &[Float], 
        extended_values: &mut [Float]
    ) {
        let [nx, ny, nz] = self.interior_shape;
        
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
    
    pub fn transfer_extended_values_to_interior_grid(
        &self,
        extended_values: &[Float],
        interior_values: &mut [Float]
    ) {
        let [nx, ny, nz] = self.interior_shape;
        
        for i_x in 0..nx {
            for i_y in 0..ny {
                for i_z in 0..nz {
                    let interior_indices = [i_x, i_y, i_z];
                    let flat_index_interior = self.flat_index_on_interior_grid(interior_indices);
                    
                    let extended_indices = self.extended_indices_from_interior_indices(interior_indices);
                    let flat_index_extended = self.flat_index_on_extended_grid(extended_indices);
                    
                    interior_values[flat_index_interior] = extended_values[flat_index_extended];
                }
            }
        }
    }

    #[inline(always)]
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
        let [nx, ny, nz] = self.interior_shape;
        
        assert!(
            nx % 2 == 0 && ny % 2 == 0 && nz % 2 == 0,
            "Cannot coarsen grid: interior cell counts must be even. Got [{}, {}, {}]",
            nx, ny, nz
        );

        let extended_shape = [
            nx / 2 + 2 * INTERIOR_OFFSET,
            ny / 2 + 2 * INTERIOR_OFFSET,
            nz / 2 + 2 * INTERIOR_OFFSET,
        ];

        let extended_stride = [
            extended_shape[1] * extended_shape[2], 
            extended_shape[2]
        ];

        let interior_shape = [
            nx / 2,
            ny / 2,
            nz / 2
        ];

        let interior_stride = [
            interior_shape[1] * interior_shape[2],
            interior_shape[2]
        ];
        
        Grid {
            start_point: self.start_point,
            cell_length: SpatialVector([
                self.cell_length[0] * 2.0,
                self.cell_length[1] * 2.0,
                self.cell_length[2] * 2.0,
            ]),
            extended_shape,
            extended_stride,
            interior_shape,
            interior_stride
        }
    }
}
