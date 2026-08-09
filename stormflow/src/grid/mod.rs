
pub mod builder;
pub mod gpu_version;
pub mod boundary_face;
pub mod parallel_loops;
pub mod finite_difference;
pub mod stencils;

use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

use gpu_version::GpuGrid;
use stencils::AxisStencils;

/// Constants that define how many ghost cells there are outside the interior cells
pub const INTERIOR_OFFSET: usize = 3;

/// Coarsening stops once a coarsened grid's interior cell count in any dimension would drop
/// to this value or below (used by geometric multigrid hierarchy construction).
pub const SMALLEST_NR_CELLS_FOR_COARSENING: usize = 2;

/// Relative tolerance used by [`Grid::is_uniform`] when checking whether all cells along an axis
/// have the same length.
const UNIFORMITY_TOLERANCE: Float = 1e-5;

#[derive(Debug, Clone)]
/// A structured cartesian grid with ghost cells. The cell length for the grid is allowed to vary, 
/// but the variation is such that it is a constant number of cells in all three directions 
/// (x, y, z). This means that for any vertices or cell center, it is straight forward to find the 
/// neighboring point or cell through simple indexing logic. The varying cell length is defined by 
/// storing the points on the interior grid as three separate arrays. The coordinates of the cell 
/// centers, as well as local cell lengths and distances from different faces etc., are calculated 
/// from these values through various methods. Since only the interior points are stored directly, 
/// the extended grid is defined implicitly. It is assumed that the cell length outside the interior 
/// grid is equal to the closest cell on the interior. That is, there is no variation in the cell 
/// length in the ghost cells. Otherwise, the grid stores the necessary values to efficiently access 
/// data in vectors that are intended to represent data on this grid (shape and stride), covering 
/// both data for both the interior and the extended grid. 
///
/// The stored points are the *cell vertices* (the faces of the interior cells), so an axis with
/// `n` interior cells stores `n + 1` points: cell `i` spans `[points[i], points[i + 1]]`, has
/// length `points[i + 1] - points[i]` and center `0.5 * (points[i] + points[i + 1])`. On a
/// staggered (MAC) layout the pressure lives at the cell centers while `velocity[i][axis]` lives
/// on cell `i`'s positive face along `axis`, i.e. exactly on one of the stored points.
pub struct Grid {
    /// Vector storing the locations of the points in the grid as three vectors, one for each 
    /// dimensions
    pub interior_points: [Vec<Float>; 3],
    /// The shape of the grid that includes the ghost cells that exists outside the interior points.
    /// That is, it stores the number of cells in x, y, and z direction. It equals the number of
    /// interior cells (one less than the length of the corresponding interior point array, since
    /// those hold the cell vertices) + 2 times the number of ghost cells, defined in the global
    /// constant INTERIOR_OFFSET
    pub extended_shape: [usize; 3],
    /// The stride to access data in a flat array that stores values including the ghost cells. The 
    /// grid is set up to increment cells in z direction first, then y, and then x. The first value 
    /// in this array gives the index increment that is needed to jump to the next value in the 
    /// x-direction, which is equal to the shape of the extended grid in y-direction multiplied by 
    /// the shape in the z-direction. The second value gives the jump to access the data on the next
    /// y-direction, which equals the shape in the y-direction. The last gives the next value in the 
    /// z-direction which is just one. 
    pub extended_stride: [usize; 3],
    /// Same as the "extended_shape", but for the interior grid
    pub interior_shape: [usize; 3],
    /// Same as the "extended_stride", but for the interior grid
    pub interior_stride: [usize; 3],
    /// Precomputed finite-difference and interpolation weights, one table set per axis, indexed by
    /// the extended index along that axis. Since the grid is a Cartesian tensor product every
    /// stencil coefficient depends on a single axis index only, so these are cheap 1D tables that
    /// the kernels read instead of deriving coefficients from a cell length that is only valid on a
    /// uniform grid. See [`AxisStencils`].
    pub stencils: [AxisStencils; 3]
}

#[inline(always)]
/// Position of the cell vertex with (possibly out-of-range) index `vertex_index`, given one axis'
/// interior points.
///
/// Vertices outside the stored interior points are extrapolated with the cell length of the
/// nearest interior cell, which is what makes the ghost region well-defined without storing it:
/// "the cell length outside the interior grid is equal to the closest cell on the interior".
fn vertex_position_along_axis(points: &[Float], vertex_index: isize) -> Float {
    let last = (points.len() - 1) as isize;

    if vertex_index < 0 {
        let cell_length = points[1] - points[0];

        points[0] + (vertex_index as Float) * cell_length
    } else if vertex_index > last {
        let cell_length = points[last as usize] - points[(last - 1) as usize];

        points[last as usize] + ((vertex_index - last) as Float) * cell_length
    } else {
        points[vertex_index as usize]
    }
}

impl Grid {
    /// Creates a grid directly from the interior cell vertices along each axis. Each array must be
    /// strictly increasing and hold at least two points (one interior cell).
    ///
    /// This is the general constructor — every other constructor is a convenience wrapper that
    /// generates the point arrays and forwards to this one.
    pub fn new_from_points(interior_points: [Vec<Float>; 3]) -> Self {
        let interior_shape = [
            interior_points[0].len().checked_sub(1).expect("grid needs at least one point in x"),
            interior_points[1].len().checked_sub(1).expect("grid needs at least one point in y"),
            interior_points[2].len().checked_sub(1).expect("grid needs at least one point in z"),
        ];

        assert!(
            interior_shape[0] > 0 && interior_shape[1] > 0 && interior_shape[2] > 0,
            "Grid needs at least one interior cell along every axis. \
             Got point counts [{}, {}, {}]",
            interior_points[0].len(), interior_points[1].len(), interior_points[2].len()
        );

        debug_assert!(
            [&interior_points[0], &interior_points[1], &interior_points[2]]
                .iter()
                .all(|points| points.windows(2).all(|w| w[1] > w[0])),
            "Grid point arrays must be strictly increasing"
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

        let stencils: [AxisStencils; 3] = std::array::from_fn(|axis| {
            AxisStencils::new(
                extended_shape[axis],
                |vertex_index| vertex_position_along_axis(&interior_points[axis], vertex_index)
            )
        });

        Self {
            interior_points,
            extended_shape,
            extended_stride,
            interior_shape,
            interior_stride,
            stencils
        }
    }

    /// Creates a grid with a constant cell length in each direction, starting at `start_point`.
    pub fn new_direct(
        start_point: SpatialVector,
        cell_length: SpatialVector,
        interior_shape: [usize; 3]
    ) -> Self {
        let axis_points = |axis: usize| -> Vec<Float> {
            (0..=interior_shape[axis])
                .map(|i| start_point[axis] + (i as Float) * cell_length[axis])
                .collect()
        };

        Self::new_from_points([axis_points(0), axis_points(1), axis_points(2)])
    }

    /// Creates a grid with a constant cell length spanning `start_point` to `end_point`.
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

    /// The GPU mirror of the grid is still built around a single, constant cell length per axis
    /// (see `gpu_version::GpuGrid` and the `.wgsl` shaders that consume it), so a grid with
    /// varying cell lengths cannot be represented on the GPU yet. This asserts rather than
    /// silently shipping the wrong metrics to the shaders.
    pub fn as_gpu_version(&self) -> GpuGrid {
        assert!(
            self.is_uniform(),
            "The GPU grid representation only supports a constant cell length per axis. \
             Use the CPU pressure solver for grids with varying cell lengths."
        );

        let start_point = self.start_point();
        let cell_length = self.uniform_cell_length();

        let inv_cell_length = SpatialVector([
            1.0 / cell_length[0],
            1.0 / cell_length[1],
            1.0 / cell_length[2],
        ]);

        let inv_cell_length_squared = SpatialVector([
            inv_cell_length[0] * inv_cell_length[0],
            inv_cell_length[1] * inv_cell_length[1],
            inv_cell_length[2] * inv_cell_length[2],
        ]);

        let inv_cell_length_squared_sum =
            inv_cell_length_squared[0] + inv_cell_length_squared[1] + inv_cell_length_squared[2];

        let poisson_diagonal = -2.0 * inv_cell_length_squared_sum;
        let poisson_diagonal4 = -2.5 * inv_cell_length_squared_sum;

        GpuGrid {
            start_point: [
                start_point[0], 
                start_point[1], 
                start_point[2], 
                0.0
            ], 
            cell_length: [
                cell_length[0], 
                cell_length[1], 
                cell_length[2], 
                0.0
            ], 
            inv_cell_length: [
                inv_cell_length[0], 
                inv_cell_length[1], 
                inv_cell_length[2], 
                0.0
            ], 
            inv_cell_length_squared: [
                inv_cell_length_squared[0], 
                inv_cell_length_squared[1], 
                inv_cell_length_squared[2], 
                0.0
            ], 
            poisson_diagonal,
            poisson_inv_diagonal: 1.0 / poisson_diagonal,
            _pad0: 0,
            _pad1: 0,
            poisson_diagonal4,
            poisson_inv_diagonal4: 1.0 / poisson_diagonal4,
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

    // ------------------------------------------------------------------------------------------
    // Grid metrics
    //
    // Everything geometric is derived from the stored interior point (cell vertex) arrays. The
    // "signed interior index" used internally below is the interior cell index extended to the
    // ghost region: ghost cells on the low side have negative indices, ghost cells on the high
    // side have indices >= interior_shape[axis]. Extended index `e` maps to signed interior index
    // `e - INTERIOR_OFFSET`.
    // ------------------------------------------------------------------------------------------

    #[inline(always)]
    /// Signed interior cell indices corresponding to the given extended indices.
    fn signed_interior_indices(extended_indices: [usize; 3]) -> [isize; 3] {
        let offset = INTERIOR_OFFSET as isize;

        [
            extended_indices[0] as isize - offset,
            extended_indices[1] as isize - offset,
            extended_indices[2] as isize - offset,
        ]
    }

    #[inline(always)]
    /// Signed interior cell indices corresponding to the given interior indices.
    fn signed_from_interior_indices(interior_indices: [usize; 3]) -> [isize; 3] {
        [
            interior_indices[0] as isize,
            interior_indices[1] as isize,
            interior_indices[2] as isize,
        ]
    }

    #[inline(always)]
    /// Position of the cell vertex with (possibly out-of-range) index `vertex_index` along `axis`.
    ///
    /// Vertices outside the stored interior points are extrapolated with the cell length of the
    /// nearest interior cell, which is what makes the ghost region well-defined without storing
    /// it: "the cell length outside the interior grid is equal to the closest cell on the
    /// interior".
    fn vertex_position(&self, axis: usize, vertex_index: isize) -> Float {
        vertex_position_along_axis(&self.interior_points[axis], vertex_index)
    }

    #[inline(always)]
    /// Length of the cell with signed interior index `index` along `axis`.
    fn cell_length_along_axis(&self, axis: usize, index: isize) -> Float {
        self.vertex_position(axis, index + 1) - self.vertex_position(axis, index)
    }

    #[inline(always)]
    /// Center coordinate of the cell with signed interior index `index` along `axis`.
    fn cell_center_along_axis(&self, axis: usize, index: isize) -> Float {
        0.5 * (self.vertex_position(axis, index) + self.vertex_position(axis, index + 1))
    }

    #[inline(always)]
    fn cell_length_signed(&self, indices: [isize; 3]) -> SpatialVector {
        SpatialVector([
            self.cell_length_along_axis(0, indices[0]),
            self.cell_length_along_axis(1, indices[1]),
            self.cell_length_along_axis(2, indices[2]),
        ])
    }

    #[inline(always)]
    /// Length of the interior cell at `interior_index` along `axis`.
    pub fn cell_length_on_axis(&self, axis: usize, interior_index: usize) -> Float {
        self.cell_length_along_axis(axis, interior_index as isize)
    }

    #[inline(always)]
    /// Center coordinate of the interior cell at `interior_index` along `axis`.
    pub fn cell_center_on_axis(&self, axis: usize, interior_index: usize) -> Float {
        self.cell_center_along_axis(axis, interior_index as isize)
    }

    #[inline(always)]
    /// The cell lengths (dx, dy, dz) of the interior cell at `interior_indices`.
    pub fn cell_length(&self, interior_indices: [usize; 3]) -> SpatialVector {
        self.cell_length_signed(Self::signed_from_interior_indices(interior_indices))
    }

    #[inline(always)]
    /// The cell lengths (dx, dy, dz) of the extended-grid cell at `extended_indices`.
    pub fn cell_length_extended(&self, extended_indices: [usize; 3]) -> SpatialVector {
        self.cell_length_signed(Self::signed_interior_indices(extended_indices))
    }

    #[inline(always)]
    fn inv_cell_length_signed(&self, indices: [isize; 3]) -> SpatialVector {
        let cell_length = self.cell_length_signed(indices);

        SpatialVector([
            1.0 / cell_length[0],
            1.0 / cell_length[1],
            1.0 / cell_length[2],
        ])
    }

    #[inline(always)]
    pub fn inv_cell_length(&self, interior_indices: [usize; 3]) -> SpatialVector {
        self.inv_cell_length_signed(Self::signed_from_interior_indices(interior_indices))
    }

    #[inline(always)]
    pub fn inv_cell_length_extended(&self, extended_indices: [usize; 3]) -> SpatialVector {
        self.inv_cell_length_signed(Self::signed_interior_indices(extended_indices))
    }

    #[inline(always)]
    fn inv_cell_length_squared_signed(&self, indices: [isize; 3]) -> SpatialVector {
        let inv_cell_length = self.inv_cell_length_signed(indices);

        SpatialVector([
            inv_cell_length[0] * inv_cell_length[0],
            inv_cell_length[1] * inv_cell_length[1],
            inv_cell_length[2] * inv_cell_length[2],
        ])
    }

    #[inline(always)]
    pub fn inv_cell_length_squared(&self, interior_indices: [usize; 3]) -> SpatialVector {
        self.inv_cell_length_squared_signed(Self::signed_from_interior_indices(interior_indices))
    }

    #[inline(always)]
    pub fn inv_cell_length_squared_extended(&self, extended_indices: [usize; 3]) -> SpatialVector {
        self.inv_cell_length_squared_signed(Self::signed_interior_indices(extended_indices))
    }

    #[inline(always)]
    pub fn cell_volume(&self, interior_indices: [usize; 3]) -> Float {
        let cell_length = self.cell_length(interior_indices);

        cell_length[0] * cell_length[1] * cell_length[2]
    }

    #[inline(always)]
    /// The 5-point, 4th order accurate second-derivative weights along `axis` for the cell at
    /// `extended_index` along that axis. Entries correspond to the neighbour offsets `-2..=2`, so
    /// entry `2` is the diagonal contribution.
    ///
    /// This is the Poisson operator the pressure solve inverts. On a uniform grid it reduces to
    /// `[-1/12, 4/3, -5/2, 4/3, -1/12] / h^2`.
    pub fn poisson_axis_stencil(&self, axis: usize, extended_index: usize) -> &[Float; 5] {
        &self.stencils[axis].second_derivative_center[extended_index]
    }

    #[inline(always)]
    /// As [`Grid::poisson_axis_stencil`], addressed by the interior index along `axis`.
    pub fn poisson_axis_stencil_interior(&self, axis: usize, interior_index: usize) -> &[Float; 5] {
        self.poisson_axis_stencil(axis, interior_index + INTERIOR_OFFSET)
    }

    #[inline(always)]
    /// Diagonal coefficient of the 2nd order accurate Poisson stencil at the given cell. On a
    /// uniform grid this is `-2 * (1/dx^2 + 1/dy^2 + 1/dz^2)`.
    pub fn poisson_diagonal(&self, interior_indices: [usize; 3]) -> Float {
        self.poisson_diagonal_extended(
            self.extended_indices_from_interior_indices(interior_indices)
        )
    }

    #[inline(always)]
    pub fn poisson_inv_diagonal(&self, interior_indices: [usize; 3]) -> Float {
        1.0 / self.poisson_diagonal(interior_indices)
    }

    #[inline(always)]
    /// Diagonal coefficient of the 4th order accurate Poisson stencil at the given cell. On a
    /// uniform grid this is `-2.5 * (1/dx^2 + 1/dy^2 + 1/dz^2)`.
    pub fn poisson_diagonal4(&self, interior_indices: [usize; 3]) -> Float {
        self.poisson_diagonal4_extended(
            self.extended_indices_from_interior_indices(interior_indices)
        )
    }

    #[inline(always)]
    pub fn poisson_inv_diagonal4(&self, interior_indices: [usize; 3]) -> Float {
        1.0 / self.poisson_diagonal4(interior_indices)
    }

    #[inline(always)]
    /// As [`Grid::poisson_diagonal`], but addressed by extended indices.
    pub fn poisson_diagonal_extended(&self, extended_indices: [usize; 3]) -> Float {
        (0..3)
            .map(|axis| {
                self.stencils[axis].second_derivative_center_low_order[extended_indices[axis]][1]
            })
            .sum()
    }

    #[inline(always)]
    /// As [`Grid::poisson_diagonal4`], but addressed by extended indices.
    pub fn poisson_diagonal4_extended(&self, extended_indices: [usize; 3]) -> Float {
        (0..3)
            .map(|axis| self.poisson_axis_stencil(axis, extended_indices[axis])[2])
            .sum()
    }

    #[inline(always)]
    /// The lower corner of the interior domain (the first stored point along each axis).
    pub fn start_point(&self) -> SpatialVector {
        SpatialVector([
            self.interior_points[0][0],
            self.interior_points[1][0],
            self.interior_points[2][0],
        ])
    }

    #[inline(always)]
    /// The upper corner of the interior domain (the last stored point along each axis).
    pub fn end_point(&self) -> SpatialVector {
        SpatialVector([
            self.interior_points[0][self.interior_points[0].len() - 1],
            self.interior_points[1][self.interior_points[1].len() - 1],
            self.interior_points[2][self.interior_points[2].len() - 1],
        ])
    }

    /// The smallest cell length found anywhere on the interior grid, over all three axes.
    pub fn min_cell_length(&self) -> Float {
        (0..3)
            .flat_map(|axis| self.interior_points[axis].windows(2).map(|w| w[1] - w[0]))
            .fold(Float::INFINITY, Float::min)
    }

    /// The largest cell length found anywhere on the interior grid, over all three axes.
    pub fn max_cell_length(&self) -> Float {
        (0..3)
            .flat_map(|axis| self.interior_points[axis].windows(2).map(|w| w[1] - w[0]))
            .fold(0.0 as Float, Float::max)
    }

    /// The cell length each axis would have if the grid were uniform: the domain length divided
    /// by the number of interior cells. Only meaningful together with [`Grid::is_uniform`].
    pub fn uniform_cell_length(&self) -> SpatialVector {
        let start_point = self.start_point();
        let end_point = self.end_point();

        SpatialVector([
            (end_point[0] - start_point[0]) / self.interior_shape[0] as Float,
            (end_point[1] - start_point[1]) / self.interior_shape[1] as Float,
            (end_point[2] - start_point[2]) / self.interior_shape[2] as Float,
        ])
    }

    /// Whether every cell along every axis has the same length (within a relative tolerance).
    pub fn is_uniform(&self) -> bool {
        (0..3).all(|axis| {
            let points = &self.interior_points[axis];
            let reference = points[1] - points[0];

            points
                .windows(2)
                .all(|w| ((w[1] - w[0]) - reference).abs() <= UNIFORMITY_TOLERANCE * reference.abs())
        })
    }

    #[inline(always)]
    /// Returns the coordinate of the cell center from the indices given. 
    pub fn cell_center(&self, interior_indices: [usize; 3]) -> SpatialVector {
        let indices = Self::signed_from_interior_indices(interior_indices);

        SpatialVector([
            self.cell_center_along_axis(0, indices[0]),
            self.cell_center_along_axis(1, indices[1]),
            self.cell_center_along_axis(2, indices[2]),
        ])
    }

    /// Returns the coordinate of the cell center from the indices given.
    pub fn cell_center_extended(&self, extended_indices: [usize; 3]) -> SpatialVector {
        // Extended index `INTERIOR_OFFSET` must land on the same physical location as interior
        // index 0, which `signed_interior_indices` takes care of: it maps that extended index
        // onto signed interior index 0.
        let indices = Self::signed_interior_indices(extended_indices);

        SpatialVector([
            self.cell_center_along_axis(0, indices[0]),
            self.cell_center_along_axis(1, indices[1]),
            self.cell_center_along_axis(2, indices[2]),
        ])
    }

    #[inline(always)]
    /// Returns the coordinate of the face center for the given interior indices and the axis
    pub fn positive_face_center(&self, interior_indices: [usize; 3], axis_index: usize) -> SpatialVector {
        let mut out = self.cell_center(interior_indices);

        out[axis_index] = self.vertex_position(axis_index, interior_indices[axis_index] as isize + 1);

        out
    }

    #[inline(always)]
    /// Returns the coordinate of the face center for the given interior indices and the axis
    pub fn negative_face_center(&self, interior_indices: [usize; 3], axis_index: usize) -> SpatialVector {
        let mut out = self.cell_center(interior_indices);

        out[axis_index] = self.vertex_position(axis_index, interior_indices[axis_index] as isize);

        out
    }

    #[inline(always)]
    /// As [`Grid::positive_face_center`], but addressed by extended indices. This is the physical
    /// location of `velocity[flat_index][axis_index]` in the staggered layout.
    pub fn positive_face_center_extended(&self, extended_indices: [usize; 3], axis_index: usize) -> SpatialVector {
        let mut out = self.cell_center_extended(extended_indices);

        let index = Self::signed_interior_indices(extended_indices)[axis_index];
        out[axis_index] = self.vertex_position(axis_index, index + 1);

        out
    }

    #[inline(always)]
    /// As [`Grid::negative_face_center`], but addressed by extended indices.
    pub fn negative_face_center_extended(&self, extended_indices: [usize; 3], axis_index: usize) -> SpatialVector {
        let mut out = self.cell_center_extended(extended_indices);

        let index = Self::signed_interior_indices(extended_indices)[axis_index];
        out[axis_index] = self.vertex_position(axis_index, index);

        out
    }
    
    /// Creates a new grid that is coarser than `self` by a factor of 2 in each dimension.
    /// 
    /// The coarse grid has exactly half the number of interior cells in each dimension: every
    /// second interior point is kept, so each coarse cell is the union of the two fine cells it
    /// replaces and the domain start point and overall extent remain the same. For a uniform grid
    /// this is exactly a doubling of the cell length.
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

        let mut coarsened_points: [Vec<Float>; 3] = [
            Vec::with_capacity(nx/2),
            Vec::with_capacity(ny/2),
            Vec::with_capacity(nz/2),
        ];

        for axis in 0..3 {
            coarsened_points[axis] = self.interior_points[axis]
                .iter()
                .step_by(2)
                .copied()
                .collect();
        }

        Self::new_from_points(coarsened_points)
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
    /// The physical coordinate along `axis` of the sample belonging to extended cell
    /// `extended_index`, for a field that is either cell-centered (`staggered == false`) or
    /// face-staggered along that axis (`staggered == true`, i.e. sitting on the cell's positive
    /// face — the convention used by `velocity[i][axis]`).
    fn sample_coordinate(&self, axis: usize, extended_index: usize, staggered: bool) -> Float {
        let index = extended_index as isize - INTERIOR_OFFSET as isize;

        if staggered {
            self.vertex_position(axis, index + 1)
        } else {
            self.cell_center_along_axis(axis, index)
        }
    }

    /// Locates `coordinate` in the (monotonically increasing) sample positions along `axis`,
    /// returning the lower extended index of the enclosing interval and the fractional weight in
    /// `[0, 1]` within it. Coordinates outside the grid are clamped to the nearest valid interval
    /// rather than extrapolated indefinitely.
    ///
    /// A binary search is used rather than the direct division that a constant cell length would
    /// allow, since the sample spacing is no longer uniform.
    fn locate_along_axis(&self, axis: usize, staggered: bool, coordinate: Float) -> (usize, Float) {
        let last = self.extended_shape[axis] - 1;

        let clamped = coordinate.clamp(
            self.sample_coordinate(axis, 0, staggered),
            self.sample_coordinate(axis, last, staggered)
        );

        let mut low = 0usize;
        let mut high = last;

        while high - low > 1 {
            let middle = low + (high - low) / 2;

            if self.sample_coordinate(axis, middle, staggered) <= clamped {
                low = middle;
            } else {
                high = middle;
            }
        }

        let index = low.min(last - 1);

        let coordinate_low = self.sample_coordinate(axis, index, staggered);
        let coordinate_high = self.sample_coordinate(axis, index + 1, staggered);

        let weight = ((clamped - coordinate_low) / (coordinate_high - coordinate_low)).clamp(0.0, 1.0);

        (index, weight)
    }

    #[inline(always)]
    /// Returns the lower-corner extended-grid indices and the fractional weights (in [0, 1] on
    /// each axis) of the trilinear stencil enclosing `point`. `staggered_axis` names the axis (if
    /// any) along which the field is face-staggered rather than cell-centered.
    fn trilinear_stencil(&self, staggered_axis: Option<usize>, point: SpatialVector) -> ([usize; 3], SpatialVector) {
        let mut i0 = [0usize; 3];
        let mut t = SpatialVector::default();

        for axis in 0..3 {
            let (index, weight) = self.locate_along_axis(
                axis,
                staggered_axis == Some(axis),
                point[axis]
            );

            i0[axis] = index;
            t[axis] = weight;
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
        let (i0, t) = self.trilinear_stencil(None, point);

        self.trilinear_gather(i0, t, |i| values[i])
    }

    /// Trilinearly interpolates the face-staggered velocity field at an arbitrary physical point.
    /// Each component is interpolated independently, since `velocity[i][axis]` physically sits at
    /// the positive face of extended cell `i` along `axis` rather than at the cell center.
    pub fn interpolate_velocity(&self, velocity: &[SpatialVector], point: SpatialVector) -> SpatialVector {
        let mut result = SpatialVector::default();

        for axis in 0..3 {
            let (i0, t) = self.trilinear_stencil(Some(axis), point);

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

#[cfg(test)]
mod tests {
    use super::*;

    fn uniform_grid() -> Grid {
        Grid::new(
            SpatialVector([-1.0, 2.0, 0.5]),
            SpatialVector([3.0, 5.0, 2.5]),
            [8, 6, 4]
        )
    }

    #[test]
    fn uniform_grid_reproduces_constant_metrics() {
        let grid = uniform_grid();

        assert!(grid.is_uniform());

        let expected = SpatialVector([0.5, 0.5, 0.5]);

        for interior_indices in [[0, 0, 0], [3, 2, 1], [7, 5, 3]] {
            let cell_length = grid.cell_length(interior_indices);

            for axis in 0..3 {
                assert!((cell_length[axis] - expected[axis]).abs() < 1e-5);
            }
        }
    }

    /// Guards the uniform-cell-length behaviour the grid had before it was generalized to varying
    /// cell lengths: every geometric quantity must still match the closed-form expression that
    /// used to be evaluated from a single stored `cell_length`/`start_point`.
    #[test]
    fn uniform_grid_matches_the_previous_closed_form_geometry() {
        let start_point = SpatialVector([-1.0, 2.0, 0.5]);
        let cell_length = SpatialVector([0.5, 0.5, 0.5]);

        let grid = uniform_grid();

        for extended_indices in [[0, 0, 0], [3, 3, 3], [5, 4, 6], [13, 11, 9]] {
            let cell_center = grid.cell_center_extended(extended_indices);

            for axis in 0..3 {
                let expected = start_point[axis]
                    + (0.5 - INTERIOR_OFFSET as Float + extended_indices[axis] as Float)
                        * cell_length[axis];

                assert!(
                    (cell_center[axis] - expected).abs() < 1e-5,
                    "cell center mismatch on axis {axis}: {} vs {}", cell_center[axis], expected
                );

                let face_center = grid.positive_face_center_extended(extended_indices, axis);

                assert!((face_center[axis] - (expected + 0.5 * cell_length[axis])).abs() < 1e-5);
            }
        }

        let inv_squared_sum: Float = (0..3).map(|axis| 1.0 / (cell_length[axis] * cell_length[axis])).sum();

        assert!((grid.poisson_diagonal([2, 2, 2]) - (-2.0 * inv_squared_sum)).abs() < 1e-3);
        assert!((grid.poisson_diagonal4([2, 2, 2]) - (-2.5 * inv_squared_sum)).abs() < 1e-3);
    }

    /// On a uniform grid every stencil table has to collapse onto the constant coefficients that
    /// were hardcoded in the kernels before the grid was generalized, so that switching to the
    /// table lookups changes nothing for uniform grids.
    #[test]
    fn uniform_grid_stencils_match_the_previous_hardcoded_coefficients() {
        let grid = uniform_grid();
        let h = 0.5 as Float;

        let close = |a: Float, b: Float| assert!(
            (a - b).abs() < 1e-3 * b.abs().max(1.0), "got {a}, expected {b}"
        );

        for axis in 0..3 {
            let stencils = &grid.stencils[axis];

            // Pick an index well inside the interior so the ghost extrapolation plays no role.
            let index = INTERIOR_OFFSET + 2;

            // interp4: (9 * (f_0 + f_1) - (f_m1 + f_2)) / 16, both staggering directions.
            for weights in [
                &stencils.interpolate_center_to_face[index],
                &stencils.interpolate_face_to_center[index],
            ] {
                close(weights[0], -1.0 / 16.0);
                close(weights[1], 9.0 / 16.0);
                close(weights[2], 9.0 / 16.0);
                close(weights[3], -1.0 / 16.0);
            }

            // The staggered 4-point derivative: (27 * (f_1 - f_0) - (f_2 - f_m1)) / (24 h).
            for weights in [
                &stencils.gradient_center_to_face[index],
                &stencils.divergence_face_to_center[index],
            ] {
                close(weights[0], 1.0 / (24.0 * h));
                close(weights[1], -27.0 / (24.0 * h));
                close(weights[2], 27.0 / (24.0 * h));
                close(weights[3], -1.0 / (24.0 * h));
            }

            // laplacian4: [-1, 16, -30, 16, -1] / (12 h^2).
            for weights in [
                &stencils.second_derivative_center[index],
                &stencils.second_derivative_face[index],
            ] {
                close(weights[0], -1.0 / (12.0 * h * h));
                close(weights[1], 16.0 / (12.0 * h * h));
                close(weights[2], -30.0 / (12.0 * h * h));
                close(weights[3], 16.0 / (12.0 * h * h));
                close(weights[4], -1.0 / (12.0 * h * h));
            }

            // upwind_derivative4_plus / _minus, both staggering directions.
            for weights in [
                &stencils.upwind_plus_center[index],
                &stencils.upwind_plus_face[index],
            ] {
                for (slot, expected) in [-1.0, 6.0, -18.0, 10.0, 3.0].into_iter().enumerate() {
                    close(weights[slot], expected / (12.0 * h));
                }
            }

            for weights in [
                &stencils.upwind_minus_center[index],
                &stencils.upwind_minus_face[index],
            ] {
                for (slot, expected) in [-3.0, -10.0, 18.0, -6.0, 1.0].into_iter().enumerate() {
                    close(weights[slot], expected / (12.0 * h));
                }
            }
        }

        // And the assembled diagonals the pressure solver reads.
        let inv_squared_sum: Float = 3.0 / (h * h);

        close(grid.poisson_diagonal4([2, 2, 2]), -2.5 * inv_squared_sum);
        close(grid.poisson_diagonal([2, 2, 2]), -2.0 * inv_squared_sum);
    }

    /// Geometrically stretched points along one axis, for the non-uniform stencil tests.
    fn stretched_points(nr_cells: usize, first_cell_length: Float, growth: Float) -> Vec<Float> {
        let mut points = vec![0.0 as Float];
        let mut cell_length = first_cell_length;

        for _ in 0..nr_cells {
            let next = points[points.len() - 1] + cell_length;

            points.push(next);
            cell_length *= growth;
        }

        points
    }

    /// The real test of the generalized stencils: on a deliberately stretched grid they must still
    /// be exact for the polynomials their node count guarantees. Every 4-node stencil here is
    /// exact up to degree 3, and every 5-node one up to degree 4.
    #[test]
    fn stretched_grid_stencils_are_exact_for_polynomials() {
        let points = stretched_points(24, 0.3, 1.08);

        let grid = Grid::new_from_points([points.clone(), points.clone(), points]);

        // f(x) = 1 - 0.7x + 0.3x^2 - 0.05x^3, plus a quartic term only the 5-node stencils see.
        let cubic = |x: Float| 1.0 - 0.7 * x + 0.3 * x * x - 0.05 * x.powi(3);
        let d_cubic = |x: Float| -0.7 + 0.6 * x - 0.15 * x * x;
        let quartic = |x: Float| cubic(x) + 0.01 * x.powi(4);
        let dd_quartic = |x: Float| 0.6 - 0.3 * x + 0.12 * x * x;

        let axis = 0;
        let stencils = &grid.stencils[axis];

        for extended_index in INTERIOR_OFFSET..(INTERIOR_OFFSET + grid.interior_shape[axis]) {
            let cell_index = extended_index as isize - INTERIOR_OFFSET as isize;

            let center = grid.cell_center_along_axis(axis, cell_index);
            let face = grid.vertex_position(axis, cell_index + 1);

            let center_at = |k: isize| grid.cell_center_along_axis(axis, cell_index + k);
            let face_at = |k: isize| grid.vertex_position(axis, cell_index + 1 + k);

            let sample4 = |f: &dyn Fn(Float) -> Float, at: &dyn Fn(isize) -> Float, first: isize| {
                std::array::from_fn::<Float, 4, _>(|n| f(at(first + n as isize)))
            };
            let sample5 = |f: &dyn Fn(Float) -> Float, at: &dyn Fn(isize) -> Float, first: isize| {
                std::array::from_fn::<Float, 5, _>(|n| f(at(first + n as isize)))
            };

            let dot4 = |w: &[Float; 4], f: [Float; 4]| (0..4).map(|n| w[n] * f[n]).sum::<Float>();
            let dot5 = |w: &[Float; 5], f: [Float; 5]| (0..5).map(|n| w[n] * f[n]).sum::<Float>();

            let tolerance = 1e-3;

            // Interpolation, both staggering directions.
            assert!(
                (dot4(&stencils.interpolate_center_to_face[extended_index],
                      sample4(&cubic, &center_at, -1)) - cubic(face)).abs() < tolerance,
                "center -> face interpolation at {extended_index}"
            );
            assert!(
                (dot4(&stencils.interpolate_face_to_center[extended_index],
                      sample4(&cubic, &face_at, -2)) - cubic(center)).abs() < tolerance,
                "face -> center interpolation at {extended_index}"
            );

            // Gradient and divergence must be exact for the same cubic, which is what makes them
            // consistent with each other on a stretched grid.
            assert!(
                (dot4(&stencils.gradient_center_to_face[extended_index],
                      sample4(&cubic, &center_at, -1)) - d_cubic(face)).abs() < tolerance,
                "gradient at {extended_index}"
            );
            assert!(
                (dot4(&stencils.divergence_face_to_center[extended_index],
                      sample4(&cubic, &face_at, -2)) - d_cubic(center)).abs() < tolerance,
                "divergence at {extended_index}"
            );

            // The Poisson / viscous second derivatives, exact up to a quartic.
            assert!(
                (dot5(&stencils.second_derivative_center[extended_index],
                      sample5(&quartic, &center_at, -2)) - dd_quartic(center)).abs() < tolerance,
                "second derivative (center) at {extended_index}"
            );
            assert!(
                (dot5(&stencils.second_derivative_face[extended_index],
                      sample5(&quartic, &face_at, -2)) - dd_quartic(face)).abs() < tolerance,
                "second derivative (face) at {extended_index}"
            );

            // Both upwind branches.
            assert!(
                (dot5(&stencils.upwind_plus_center[extended_index],
                      sample5(&cubic, &center_at, -3)) - d_cubic(center)).abs() < tolerance,
                "upwind + at {extended_index}"
            );
            assert!(
                (dot5(&stencils.upwind_minus_face[extended_index],
                      sample5(&cubic, &face_at, -1)) - d_cubic(face)).abs() < tolerance,
                "upwind - at {extended_index}"
            );
        }
    }

    #[test]
    fn ghost_cells_inherit_the_nearest_interior_cell_length() {
        let grid = Grid::new_from_points([
            vec![0.0, 1.0, 3.0, 6.0],
            vec![0.0, 1.0, 2.0],
            vec![0.0, 2.0, 3.0]
        ]);

        // Low-side ghost cells copy the first interior cell's length (1.0), high-side ghosts copy
        // the last one's (3.0).
        assert!((grid.cell_length_extended([0, INTERIOR_OFFSET, INTERIOR_OFFSET])[0] - 1.0).abs() < 1e-6);
        assert!((grid.cell_length_extended([INTERIOR_OFFSET + 3, INTERIOR_OFFSET, INTERIOR_OFFSET])[0] - 3.0).abs() < 1e-6);

        // ... and the ghost cell centers continue on from the domain edge with that length.
        assert!((grid.cell_center_extended([INTERIOR_OFFSET - 1, INTERIOR_OFFSET, INTERIOR_OFFSET])[0] + 0.5).abs() < 1e-6);
    }

    #[test]
    fn cell_centers_and_faces_are_consistent() {
        let grid = Grid::new_from_points([
            vec![0.0, 1.0, 3.0, 6.0],
            vec![0.0, 1.0, 2.0],
            vec![0.0, 2.0, 3.0],
        ]);

        for i in 0..3 {
            let center = grid.cell_center([i, 0, 0])[0];
            let negative = grid.negative_face_center([i, 0, 0], 0)[0];
            let positive = grid.positive_face_center([i, 0, 0], 0)[0];

            assert!((center - 0.5 * (negative + positive)).abs() < 1e-6);
            assert!((positive - negative - grid.cell_length([i, 0, 0])[0]).abs() < 1e-6);
        }
    }

    #[test]
    fn coarsening_merges_neighboring_cells() {
        let grid = Grid::new_from_points([
            vec![0.0, 1.0, 3.0, 6.0, 10.0],
            vec![0.0, 1.0, 2.0],
            vec![0.0, 2.0, 3.0],
        ]);

        let coarse = grid.coarsened();

        assert_eq!(coarse.interior_shape, [2, 1, 1]);
        assert_eq!(coarse.interior_points[0], vec![0.0, 3.0, 10.0]);
        assert!((coarse.start_point()[0] - grid.start_point()[0]).abs() < 1e-6);
        assert!((coarse.end_point()[0] - grid.end_point()[0]).abs() < 1e-6);
    }

    #[test]
    fn interpolation_recovers_a_linear_field() {
        let grid = Grid::new_from_points([
            vec![0.0, 1.0, 3.0, 6.0, 10.0],
            vec![0.0, 1.0, 2.5, 4.0],
            vec![0.0, 2.0, 3.0, 7.0],
        ]);

        let values: Vec<Float> = (0..grid.nr_extended_cells())
            .map(|i| {
                let center = grid.cell_center_extended(grid.extended_indices_from_flat_index(i));

                2.0 * center[0] - 3.0 * center[1] + 0.5 * center[2]
            })
            .collect();

        let point = SpatialVector([4.2, 1.7, 2.4]);
        let expected = 2.0 * point[0] - 3.0 * point[1] + 0.5 * point[2];

        assert!((grid.interpolate_cell_centered_scalar(&values, point) - expected).abs() < 1e-3);
    }
}
