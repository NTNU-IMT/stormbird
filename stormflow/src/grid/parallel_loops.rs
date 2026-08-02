use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

use rayon::prelude::*;

use super::{Grid, INTERIOR_OFFSET};

impl Grid {
    /// Iterates over the **interior** cells of `self` in parallel and executes a kernel closure
    /// for each cell, writing the result back into `out` (sized to `self.nr_interior_cells()`).
    ///
    /// The kernel receives the flat interior index, the `[i, j, k]` interior indices (cheap to
    /// hand over here since they fall out of the loop nest for free, sparing kernels the div/mod
    /// that `interior_indices_from_flat_index` would otherwise cost per cell), and the current
    /// value at that index in `out` — useful for kernels that add a correction rather than
    /// overwrite (see `MultigridCPU::prolongate_and_correct`).
    ///
    /// Cache-friendly like `parallel_spatial_vector_update`: parallelizes over x-planes (chunks
    /// of `interior_stride[0]`) so each Rayon task walks a single, contiguous, sequential run of
    /// memory rather than a strided one.
    pub fn parallel_interior_update<F>(
        &self,
        out: &mut [Float],
        kernel: F,
    )
    where
        F: Fn(usize, [usize; 3], Float) -> Float + Sync,
    {
        let [_nx, ny, nz] = self.interior_shape;
        let plane = self.interior_stride[0];

        out.par_chunks_mut(plane)
            .enumerate()
            .for_each(|(ii, out_plane)| {
                // `local`/`idx` both walk their respective plane contiguously (k is the fastest-
                // varying, stride-1 axis, j the next), so each cell is reached by a plain
                // increment rather than by re-deriving it from `ii`/`ji`/`ki` with a multiply-add
                // every time.
                let mut idx = ii * plane;
                let mut local = 0usize;

                for ji in 0..ny {
                    for ki in 0..nz {
                        out_plane[local] = kernel(idx, [ii, ji, ki], out_plane[local]);

                        idx += 1;
                        local += 1;
                    }
                }
            });
    }

    /// Writes each interior cell's value into its corresponding position on the **extended**
    /// grid of `out` (sized to `self.nr_extended_cells()`), running in parallel over interior
    /// x-planes.
    ///
    /// Chunks `out` into x-planes of the extended grid and skips the `INTERIOR_OFFSET` ghost planes on each side —
    /// the same skip/take pattern `parallel_spatial_vector_update` uses. Within a plane, the
    /// extended write position only ever moves by `+1` along `k` (the fastest-varying axis) and
    /// by `+= extended_stride[1]` at the start of each `j` row, so — like `parallel_interior_update`
    /// — it's tracked with running counters instead of being recomputed from `[i, j, k]` via
    /// `flat_index_on_extended_grid_from_interior_indices` on every cell.
    pub fn parallel_interior_to_extended<F>(
        &self,
        out: &mut [Float],
        kernel: F,
    )
    where
        F: Fn(usize) -> Float + Sync,
    {
        let [nxi, nyi, nzi] = self.interior_shape;
        let interior_plane = self.interior_stride[0];
        let extended_plane = self.extended_stride[0];
        let extended_row_stride = self.extended_stride[1];

        out.par_chunks_mut(extended_plane)
            .enumerate()
            .skip(INTERIOR_OFFSET)
            .take(nxi)
            .for_each(|(i_ext, out_plane)| {
                let ii = i_ext - INTERIOR_OFFSET;

                let mut idx_interior = ii * interior_plane;
                let mut row_start = extended_row_stride * INTERIOR_OFFSET + INTERIOR_OFFSET;

                for _ji in 0..nyi {
                    let mut local = row_start;

                    for _ki in 0..nzi {
                        out_plane[local] = kernel(idx_interior);

                        idx_interior += 1;
                        local += 1;
                    }

                    row_start += extended_row_stride;
                }
            });
    }

    /// Iterates over the interior indices of `self` in parallel and executes a kernel closure for
    /// each index. The input to the kernel is both the index on the extended grid and the
    /// current value of the out vector, in case it is needed for the update.
    pub fn parallel_spatial_vector_update<F>(
        &self,
        out: &mut [SpatialVector],
        kernel: F
    )
    where
        F: Fn(usize, SpatialVector) -> SpatialVector + Sync,
    {
        let [nxi, nyi, nzi] = self.interior_shape;
        let [_nx, ny, nz] = self.extended_shape;
        let plane = ny * nz;

        out.par_chunks_mut(plane)
            .enumerate()
            .skip(INTERIOR_OFFSET)
            .take(nxi)
            .for_each(|(i, out_plane)| {
                for ji in 0..nyi {
                    let j = ji + INTERIOR_OFFSET;
                    let mut i_extended = self.flat_index_on_extended_grid([i, j, INTERIOR_OFFSET]);

                    for _k in 0..nzi {
                        let local = i_extended - i * plane;

                        out_plane[local] = kernel(i_extended, out_plane[local]);

                        i_extended += 1;
                    }
                }
            });
    }
}
