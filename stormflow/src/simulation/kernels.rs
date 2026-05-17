
use crate::grid::Grid;

use stormath::type_aliases::Float;
use stormath::spatial_vector::SpatialVector;

pub fn convective_term(
    i_flat_interior: usize,
    grid: &Grid,
    velocity: &[SpatialVector], 
) -> [Float; 3] {
    let interior_indices = grid.interior_indices_from_flat_index(i_flat_interior);
    
    let [i, j, k] = grid.extended_indices_from_interior_indices(interior_indices);
    
    let i_0 = grid.flat_index_on_extended_grid([i, j, k]);
    
    let i_p = [
        grid.flat_index_on_extended_grid([i+1, j, k]),
        grid.flat_index_on_extended_grid([i, j+1, k]),
        grid.flat_index_on_extended_grid([i, j, k+1])
    ];
    
    let i_n = [
        grid.flat_index_on_extended_grid([i-1, j, k]),
        grid.flat_index_on_extended_grid([i, j-1, k]),
        grid.flat_index_on_extended_grid([i, j, k-1])
    ];

    let mut out = [0.0; 3];
    
    for vel_component in 0..3 {
        let mut indices_p_i = [i, j, k];
        indices_p_i[vel_component] += 1; // Indices to neighbor cell relative to u_i
        
        let i_p_i = grid.flat_index_on_extended_grid(indices_p_i);
        
        for der_dir in 0..3 {
            let u_j = if vel_component == der_dir {
                velocity[i_0][der_dir]
            } else {
                let mut indices_pn_i = indices_p_i.clone();
                indices_pn_i[der_dir] -= 1;
                let i_pn = grid.flat_index_on_extended_grid(indices_pn_i);
                
                0.25 * (
                    velocity[i_0][der_dir] + // Current cell
                    velocity[i_p_i][der_dir] + // Neighbor, in the u_i direction
                    velocity[i_n[der_dir]][der_dir] + // Current cell, opposite face
                    velocity[i_pn][der_dir] // Neighbor, in the u_i direction, opposite face
                )
            };
            
            let u_i = velocity[i_0][vel_component];
            
            let dui_dxj = if u_j > 0.0 {
                let u_i_n = velocity[i_n[der_dir]][vel_component];
                
                (u_i - u_i_n) / grid.cell_length[der_dir]
            } else {
                let u_i_p = velocity[i_p[der_dir]][vel_component];
                
                (u_i_p - u_i) / grid.cell_length[der_dir]
            };
                
            out[vel_component] -= u_j * dui_dxj; 
        }
    }

    out
}

pub fn viscous_term(
    i_flat_interior: usize,
    grid: &Grid,
    velocity: &[SpatialVector],
    viscosity: Float
) -> [Float; 3] {
    let mut out = [0.0; 3];
    
    let interior_indices = grid.interior_indices_from_flat_index(i_flat_interior);
    
    let [i, j, k] = grid.extended_indices_from_interior_indices(interior_indices);
    
    let i_0 = grid.flat_index_on_extended_grid([i, j, k]);
    
    for vel_comp in 0..3 {
        for deriv_dir in 0..3 {
            let mut indices_p = [i, j, k];
            indices_p[deriv_dir] += 1;
            let i_p = grid.flat_index_on_extended_grid(indices_p);
            
            let mut indices_n = [i, j, k];
            indices_n[deriv_dir] -= 1;
            let i_n = grid.flat_index_on_extended_grid(indices_n);
            
            out[vel_comp] += viscosity * (
                velocity[i_p][vel_comp] - 
                2.0 * velocity[i_0][vel_comp] + 
                velocity[i_n][vel_comp]
            ) / grid.cell_length[deriv_dir].powi(2);
        }
    }
    
    out
}

pub fn body_force_term(
    i_flat_interior: usize,
    grid: &Grid,
    body_force: &[SpatialVector],
    density: Float
) -> [Float; 3] {

    let mut out = [0.0; 3];
    
    let interior_indices = grid.interior_indices_from_flat_index(i_flat_interior);
    
    let [i, j, k] = grid.extended_indices_from_interior_indices(interior_indices);
    
    let i_0 = grid.flat_index_on_extended_grid([i, j, k]);
    
    for a_i in 0..3 {
        let mut indices_p = [i, j, k];
        indices_p[a_i] += 1;
        let i_p = grid.flat_index_on_extended_grid(indices_p);
        
        out[a_i] += 0.5 * (body_force[i_0][a_i] + body_force[i_p][a_i]) / density
    }
    
    out
}