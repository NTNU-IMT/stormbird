
use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

pub const INTERIOR_OFFSET: usize = 1;

#[derive(Debug, Clone)]
/// Structured grid definition
pub struct Grid {
    pub start_point: SpatialVector,
    pub cell_length: SpatialVector,
    pub inv_cell_length: SpatialVector,
    pub inv_cell_length_squared: SpatialVector,
    pub poisson_diagonal: Float,
    pub extended_shape: [usize; 3],
    pub extended_stride: [usize; 3],
    pub interior_shape: [usize; 3],
    pub interior_stride: [usize; 3]
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

        let inv_cell_length = SpatialVector([
            1.0 / cell_length[0],
            1.0 / cell_length[1],
            1.0 / cell_length[2]
        ]);

        let inv_cell_length_squared = SpatialVector([
            inv_cell_length[0].powi(2),
            inv_cell_length[1].powi(2),
            inv_cell_length[2].powi(2),
        ]);

        let poisson_diagonal = 1.0 / (-2.0 * (
            inv_cell_length_squared[0] + 
            inv_cell_length_squared[1] + 
            inv_cell_length_squared[2])
        );
        
        let extended_shape = [
            interior_shape[0] + 2 * INTERIOR_OFFSET,
            interior_shape[1] + 2 * INTERIOR_OFFSET,
            interior_shape[2] + 2 * INTERIOR_OFFSET,
        ];

        let extended_stride = [
            extended_shape[1] * extended_shape[2], 
            extended_shape[2],
            1usize
        ];

        let interior_stride = [
            interior_shape[1] * interior_shape[2],
            interior_shape[2],
            1usize
        ];

        Self {
            start_point,
            cell_length,
            inv_cell_length,
            inv_cell_length_squared,
            poisson_diagonal,
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
    pub fn interior_indices_from_flat_index(&self, flat_index: usize) -> [usize; 3] {        
        let ix = flat_index / self.interior_stride[0];
        let iy = (flat_index % self.interior_stride[0]) / self.interior_stride[1];
        let iz = flat_index % self.interior_stride[1];
        
        [ix, iy, iz]
    }

    #[inline(always)]
    pub fn extended_indices_from_flat_index(&self, flat_index: usize) -> [usize; 3] {
        let ix = flat_index / self.extended_stride[0];
        let iy = (flat_index % self.extended_stride[0]) / self.extended_stride[1];
        let iz = flat_index % self.extended_stride[1];
        
        [ix, iy, iz]
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

    /// Returns the coordinate of the cell center from the indices given. 
    pub fn cell_center_extended(&self, extended_indices: [usize; 3]) -> SpatialVector {
        let start_cell_center = self.start_point - 0.5 * self.cell_length;
        
        SpatialVector(
            [
                start_cell_center[0] + (extended_indices[0] as Float) * self.cell_length[0],
                start_cell_center[1] + (extended_indices[1] as Float) * self.cell_length[1],
                start_cell_center[2] + (extended_indices[2] as Float) * self.cell_length[2],
            ]
        )
    }

    #[inline(always)]
    /// Returns the coordinate of the face center for the given interior indices and the axis
    pub fn positive_face_center(&self, interior_indices: [usize; 3], axis_index: usize) -> SpatialVector {
        let mut out = self.cell_center(interior_indices);

        out[axis_index] += 0.5 * self.cell_length[axis_index];

        out
    }

    #[inline(always)]
    /// Returns the coordinate of the face center for the given interior indices and the axis
    pub fn negative_face_center(&self, interior_indices: [usize; 3], axis_index: usize) -> SpatialVector {
        let mut out = self.cell_center(interior_indices);

        out[axis_index] -= 0.5 * self.cell_length[axis_index];

        out
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
            extended_shape[2],
            1usize
        ];

        let interior_shape = [
            nx / 2,
            ny / 2,
            nz / 2
        ];

        let interior_stride = [
            interior_shape[1] * interior_shape[2],
            interior_shape[2],
            1usize
        ];

        let cell_length = SpatialVector([
            self.cell_length[0] * 2.0,
            self.cell_length[1] * 2.0,
            self.cell_length[2] * 2.0,
        ]);

        let inv_cell_length = SpatialVector([
            1.0 / cell_length[0],
            1.0 / cell_length[1],
            1.0 / cell_length[2]
        ]);

        let inv_cell_length_squared = SpatialVector([
            inv_cell_length[0].powi(2),
            inv_cell_length[1].powi(2),
            inv_cell_length[2].powi(2),
        ]);

        let poisson_diagonal = 1.0 / (-2.0 * (
            inv_cell_length_squared[0] + 
            inv_cell_length_squared[1] + 
            inv_cell_length_squared[2])
        );
        
        Grid {
            start_point: self.start_point,
            cell_length,
            inv_cell_length,
            inv_cell_length_squared,
            poisson_diagonal,
            extended_shape,
            extended_stride,
            interior_shape,
            interior_stride
        }
    }
}