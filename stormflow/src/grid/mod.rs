
pub mod gpu_version;
pub mod boundary_face;
pub mod parallel_loops;

use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

use gpu_version::GpuGrid;

pub const INTERIOR_OFFSET: usize = 3;

/// Coarsening stops once a coarsened grid's interior cell count in any dimension would drop
/// to this value or below (used by geometric multigrid hierarchy construction).
pub const SMALLEST_NR_CELLS_FOR_COARSENING: usize = 2;

#[derive(Debug, Clone)]
/// Structured grid definition
pub struct Grid {
    pub start_point: SpatialVector,
    pub cell_length: SpatialVector,
    pub inv_cell_length: SpatialVector,
    pub inv_cell_length_squared: SpatialVector,
    pub poisson_diagonal: Float,
    pub poisson_inv_diagonal: Float,
    /// Diagonal coefficient of the 4th order accurate (5-point-per-axis) discrete Laplacian, used
    /// by `MultigridCPU`/`MultigridGPU`. Kept separate from `poisson_diagonal` (the 2nd order
    /// value) since `FftCPU` still relies on the 2nd order stencil's diagonalization by DCT/DST.
    pub poisson_diagonal4: Float,
    pub poisson_inv_diagonal4: Float,
    pub extended_shape: [usize; 3],
    pub extended_stride: [usize; 3],
    pub interior_shape: [usize; 3],
    pub interior_stride: [usize; 3]
}

impl Grid {
    pub fn new_direct(
        start_point: SpatialVector,
        cell_length: SpatialVector,
        interior_shape: [usize; 3]
    ) -> Self {
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

        let poisson_diagonal = -2.0 * (
            inv_cell_length_squared[0] + 
            inv_cell_length_squared[1] + 
            inv_cell_length_squared[2]
        );

        let poisson_inv_diagonal = 1.0 / poisson_diagonal;

        let poisson_diagonal4 = -2.5 * (
            inv_cell_length_squared[0] +
            inv_cell_length_squared[1] +
            inv_cell_length_squared[2]
        );

        let poisson_inv_diagonal4 = 1.0 / poisson_diagonal4;

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
            poisson_inv_diagonal,
            poisson_diagonal4,
            poisson_inv_diagonal4,
            extended_shape,
            extended_stride,
            interior_shape,
            interior_stride
        }
    }

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

        Self::new_direct(start_point, cell_length, interior_shape)        
    }

    pub fn as_gpu_version(&self) -> GpuGrid {
        GpuGrid {
            start_point: [
                self.start_point[0], 
                self.start_point[1], 
                self.start_point[2], 
                0.0
            ], 
            cell_length: [
                self.cell_length[0], 
                self.cell_length[1], 
                self.cell_length[2], 
                0.0
            ], 
            inv_cell_length: [
                self.inv_cell_length[0], 
                self.inv_cell_length[1], 
                self.inv_cell_length[2], 
                0.0
            ], 
            inv_cell_length_squared: [
                self.inv_cell_length_squared[0], 
                self.inv_cell_length_squared[1], 
                self.inv_cell_length_squared[2], 
                0.0
            ], 
            poisson_diagonal: self.poisson_diagonal,
            poisson_inv_diagonal: self.poisson_inv_diagonal,
            _pad0: 0,
            _pad1: 0,
            poisson_diagonal4: self.poisson_diagonal4,
            poisson_inv_diagonal4: self.poisson_inv_diagonal4,
            _pad2: 0,
            _pad3: 0,
            extended_shape: [
                self.extended_shape[0] as u32, 
                self.extended_shape[1] as u32, 
                self.extended_shape[2] as u32, 
                0
            ], 
            extended_stride: [
                self.extended_stride[0] as u32, 
                self.extended_stride[1] as u32, 
                self.extended_stride[2] as u32, 
                0
            ], 
            interior_shape: [
                self.interior_shape[0] as u32, 
                self.interior_shape[1] as u32, 
                self.interior_shape[2] as u32, 
                0
            ], 
            interior_stride: [
                self.interior_stride[0] as u32, 
                self.interior_stride[1] as u32, 
                self.interior_stride[2] as u32, 
                0
            ]
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
        // Extended index `INTERIOR_OFFSET` must land on the same physical location as interior
        // index 0 (`cell_center`'s `start_point + 0.5 * cell_length`), so the origin here is
        // shifted back by `INTERIOR_OFFSET` cells rather than by a hardcoded single ghost layer.
        let start_cell_center = self.start_point + (0.5 - INTERIOR_OFFSET as Float) * self.cell_length;
        
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

        let cell_length = SpatialVector([
            self.cell_length[0] * 2.0,
            self.cell_length[1] * 2.0,
            self.cell_length[2] * 2.0,
        ]);

        let interior_shape = [
            nx / 2,
            ny / 2,
            nz / 2
        ];

        Self::new_direct(self.start_point, cell_length, interior_shape)
    }

    /// Builds the hierarchy of grids used by geometric multigrid solvers, from the finest grid
    /// (index 0, identical to `self`) down to the coarsest grid that can still be reached by
    /// repeated even-factor-of-2 coarsening.
    pub fn multigrid_hierarchy(&self) -> Vec<Grid> {
        let mut grids: Vec<Grid> = Vec::new();

        let mut current_grid = self.clone();
        let mut grid_can_get_coarser = true;

        while grid_can_get_coarser {
            grids.push(current_grid.clone());

            let interior_shape_current = current_grid.interior_shape;

            if !interior_shape_current[0].is_multiple_of(2) ||
                !interior_shape_current[1].is_multiple_of(2)||
                !interior_shape_current[2].is_multiple_of(2) {
                    grid_can_get_coarser = false
            } else {
                let coarser_grid = current_grid.coarsened();
                let interior_shape = coarser_grid.interior_shape;

                if interior_shape[0] > SMALLEST_NR_CELLS_FOR_COARSENING &&
                    interior_shape[1] > SMALLEST_NR_CELLS_FOR_COARSENING &&
                    interior_shape[2] > SMALLEST_NR_CELLS_FOR_COARSENING {
                    current_grid = coarser_grid
                } else {
                    grid_can_get_coarser = false
                }
            }
        }

        grids
    }

    #[inline(always)]
    /// Returns the lower-corner extended-grid indices and the fractional weights (in [0, 1] on
    /// each axis) of the trilinear stencil enclosing `point`, for a field whose extended index
    /// `[0, 0, 0]` sits at physical location `field_origin`. Points outside the grid are clamped
    /// to the nearest valid stencil rather than extrapolated indefinitely.
    fn trilinear_stencil(&self, field_origin: SpatialVector, point: SpatialVector) -> ([usize; 3], SpatialVector) {
        let mut i0 = [0usize; 3];
        let mut t = SpatialVector::default();

        for axis in 0..3 {
            let raw_index = (point[axis] - field_origin[axis]) * self.inv_cell_length[axis];
            let clamped_index = raw_index.clamp(0.0, (self.extended_shape[axis] - 1) as Float);

            let floor_index = (clamped_index.floor() as usize).min(self.extended_shape[axis] - 2);

            i0[axis] = floor_index;
            t[axis] = clamped_index - floor_index as Float;
        }

        (i0, t)
    }

    #[inline(always)]
    /// Blends the 8 corner values of a trilinear stencil (as returned by `trilinear_stencil`)
    /// using `sample` to fetch the scalar value at a given extended flat index.
    fn trilinear_gather(
        &self, i0: [usize; 3], 
        t: SpatialVector, 
        sample: impl Fn(usize) -> Float
    ) -> Float {
        let i1 = [i0[0] + 1, i0[1] + 1, i0[2] + 1];

        let c000 = sample(self.flat_index_on_extended_grid([i0[0], i0[1], i0[2]]));
        let c100 = sample(self.flat_index_on_extended_grid([i1[0], i0[1], i0[2]]));
        let c010 = sample(self.flat_index_on_extended_grid([i0[0], i1[1], i0[2]]));
        let c110 = sample(self.flat_index_on_extended_grid([i1[0], i1[1], i0[2]]));
        let c001 = sample(self.flat_index_on_extended_grid([i0[0], i0[1], i1[2]]));
        let c101 = sample(self.flat_index_on_extended_grid([i1[0], i0[1], i1[2]]));
        let c011 = sample(self.flat_index_on_extended_grid([i0[0], i1[1], i1[2]]));
        let c111 = sample(self.flat_index_on_extended_grid([i1[0], i1[1], i1[2]]));

        let c00 = c000 * (1.0 - t[0]) + c100 * t[0];
        let c10 = c010 * (1.0 - t[0]) + c110 * t[0];
        let c01 = c001 * (1.0 - t[0]) + c101 * t[0];
        let c11 = c011 * (1.0 - t[0]) + c111 * t[0];

        let c0 = c00 * (1.0 - t[1]) + c10 * t[1];
        let c1 = c01 * (1.0 - t[1]) + c11 * t[1];

        c0 * (1.0 - t[2]) + c1 * t[2]
    }

    /// Trilinearly interpolates a scalar field stored at extended-grid cell centers (e.g. a
    /// signed distance function) at an arbitrary physical point.
    pub fn interpolate_cell_centered_scalar(&self, values: &[Float], point: SpatialVector) -> Float {
        let field_origin = self.cell_center_extended([0, 0, 0]);
        let (i0, t) = self.trilinear_stencil(field_origin, point);

        self.trilinear_gather(i0, t, |i| values[i])
    }

    /// Trilinearly interpolates the face-staggered velocity field at an arbitrary physical point.
    /// Each component is interpolated independently, since `velocity[i][axis]` physically sits at
    /// the positive face of extended cell `i` along `axis` rather than at the cell center.
    pub fn interpolate_velocity(&self, velocity: &[SpatialVector], point: SpatialVector) -> SpatialVector {
        let mut result = SpatialVector::default();

        for axis in 0..3 {
            let mut field_origin = self.cell_center_extended([0, 0, 0]);
            field_origin[axis] += 0.5 * self.cell_length[axis];

            let (i0, t) = self.trilinear_stencil(field_origin, point);

            result[axis] = self.trilinear_gather(i0, t, |i| velocity[i][axis]);
        }

        result
    }

    pub fn cell_centered_value_from_face_staggered(
        &self,
        interior_indices: [usize; 3],
        staggered_value: &[SpatialVector]
    ) -> SpatialVector {
        let [i, j, k] = self.extended_indices_from_interior_indices(interior_indices);
        
        let i_0 = self.flat_index_on_extended_grid([i, j, k]);
        
        let i_n = [
            self.flat_index_on_extended_grid([i-1, j, k]),
            self.flat_index_on_extended_grid([i, j-1, k]),
            self.flat_index_on_extended_grid([i, j, k-1])
        ];
        
        let u = 0.5 * (staggered_value[i_0][0] + staggered_value[i_n[0]][0]);
        let v = 0.5 * (staggered_value[i_0][1] + staggered_value[i_n[1]][1]);
        let w = 0.5 * (staggered_value[i_0][2] + staggered_value[i_n[2]][2]);
        
        SpatialVector([u, v, w])
    }
}
