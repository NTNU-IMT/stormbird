
use stormath::spatial_vector::SpatialVector;

use crate::grid::Grid;

use rayon::prelude::*;

/// Function that iterates over the interior indices of a grid and executes a kernel closure for 
/// each index. The input to the kernel is both the index on the extended grid and the current value 
/// of the out vector, in case it is needed for the update
pub fn parallel_spatial_vector_update<F>(
    out: &mut [SpatialVector],
    grid: &Grid,
    kernel: F)
where
    F: Fn(usize, SpatialVector) -> SpatialVector + Sync,
{
    let [nxi, nyi, nzi] = grid.interior_shape;
    let [_nx, ny, nz] = grid.extended_shape;
    let plane = ny * nz;

    out.par_chunks_mut(plane)
        .enumerate()
        .skip(1)
        .take(nxi)
        .for_each(|(i, out_plane)| {
            for ji in 0..nyi {
                let j = ji + 1;
                let mut i_extended = grid.flat_index_on_extended_grid([i, j, 1]);

                for _k in 0..nzi {
                    let local = i_extended - i * plane;
                    
                    out_plane[local] = kernel(i_extended, out_plane[local]);

                    i_extended += 1;
                }
            }
        });
}