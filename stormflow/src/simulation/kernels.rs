
use crate::grid::Grid;

use stormath::type_aliases::Float;
use stormath::spatial_vector::SpatialVector;

#[inline(always)]
pub fn convect_and_diffuse(
    i_flat_interior: usize,
    grid: &Grid,
    velocity: &[SpatialVector],
    body_force: &[SpatialVector],
    viscosity: Float,
    density: Float
) -> SpatialVector {
    let interior_indices = grid.interior_indices_from_flat_index(i_flat_interior);
    
    let [i, j, k] = grid.extended_indices_from_interior_indices(interior_indices);
    
    let i_0 = grid.flat_index_on_extended_grid([i, j, k]);

    let i_p = [i_0 + grid.extended_stride[0], i_0 + grid.extended_stride[1], i_0 + grid.extended_stride[2]];
    let i_n = [i_0 - grid.extended_stride[0], i_0 - grid.extended_stride[1], i_0 - grid.extended_stride[2]];

    let mut out = SpatialVector::default();

    let v0 = velocity[i_0];

    let inv_density = 1.0 / density;

    // ---------------- Convective term --------------
    for vel_comp in 0..3 {
        let mut indices_p_i = [i, j, k];
        indices_p_i[vel_comp] += 1; // Indices to neighbor cell relative to u_i
        
        let i_p_i = grid.flat_index_on_extended_grid(indices_p_i);
        
        for deriv_dir in 0..3 {
            let u_i = v0[vel_comp];
            
            let u_j = if vel_comp == deriv_dir {
                v0[deriv_dir]
            } else {
                let i_pn = i_0 + grid.extended_stride[vel_comp] - grid.extended_stride[deriv_dir];
                
                0.25 * (
                    v0[deriv_dir] + // Current cell
                    velocity[i_p_i][deriv_dir] + // Neighbor, in the u_i direction
                    velocity[i_n[deriv_dir]][deriv_dir] + // Current cell, opposite face
                    velocity[i_pn][deriv_dir] // Neighbor, in the u_i direction, opposite face
                )
            };
            
            let dui_dxj = if u_j > 0.0 {
                let u_i_n = velocity[i_n[deriv_dir]][vel_comp];
                
                (u_i - u_i_n) * grid.inv_cell_length[deriv_dir]
            } else {
                let u_i_p = velocity[i_p[deriv_dir]][vel_comp];
                
                (u_i_p - u_i) * grid.inv_cell_length[deriv_dir]
            };
                
            out[vel_comp] -= u_j * dui_dxj; 

            out[vel_comp] += viscosity * (
                velocity[i_p[deriv_dir]][vel_comp] - 
                2.0 * u_i + 
                velocity[i_n[deriv_dir]][vel_comp]
            ) * grid.inv_cell_length_squared[deriv_dir];
        }

        out[vel_comp] += 0.5 * (
            body_force[i_0][vel_comp] + body_force[i_p[vel_comp]][vel_comp]
        ) * inv_density;
    }

    out
}