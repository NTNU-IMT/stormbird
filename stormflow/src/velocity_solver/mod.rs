pub mod boundary_condisitions;
pub mod kernels;
pub mod slip_mirror_stencils;
pub mod no_slip_corrections;

use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

use rayon::prelude::*;

use crate::grid::Grid;
use boundary_condisitions::VelocityBoundaryConditions;
use slip_mirror_stencils::SlipMirrorStencils;
use no_slip_corrections::NoSlipCorrections;

use kernels::{
    correct_velocities_for_geometry::{
        correct_no_slip_entry_kernel,
        correct_slip_mirror_entry_kernel
    },
    convect_and_diffuse::convect_and_diffuse_kernel,
    add_pressure_gradient::add_pressure_gradient_kernel,
    pressure_rhs::pressure_rhs_kernel
};

pub struct VelocitySolver {
    pub velocity_org: Vec<SpatialVector>,
    pub velocity_star: Vec<SpatialVector>,
    pub velocity: Vec<SpatialVector>,
    pub body_force: Vec<SpatialVector>,
    pub signed_distance_function: Vec<Float>,
    pub signed_distance_function_slip: Vec<Float>,
    pub normals_slip_surfaces: Vec<SpatialVector>,
    pub no_slip_corrections: NoSlipCorrections,
    pub slip_mirror_stencils: SlipMirrorStencils,
    pub boundary_conditions: VelocityBoundaryConditions,
    pub viscosity: Float,
    pub density: Float,
}

impl VelocitySolver {
    pub fn initialize_after_build(&mut self, grid: &Grid) {
        Self::correct_velocities_for_geometry(
            grid,
            &self.no_slip_corrections,
            &self.slip_mirror_stencils,
            &mut self.velocity
        );

        Self::correct_velocities_for_geometry(
            grid,
            &self.no_slip_corrections,
            &self.slip_mirror_stencils,
            &mut self.velocity_star
        );
    }

    pub fn initialize_before_step(&mut self, grid: &Grid) {
        self.boundary_conditions.set_ghost_cells(
            grid,
            &mut self.velocity
        );

        self.velocity_org.copy_from_slice(&self.velocity);
    }

    pub fn update_velocity_star(
        &mut self,
        grid: &Grid,
        time_step: Float
    ) {
        let inv_density = 1.0 / self.density;

        grid.parallel_spatial_vector_update(
            &mut self.velocity_star,
            |i, _current| convect_and_diffuse_kernel(
                i,
                grid,
                &self.velocity_org,
                &self.velocity,
                &self.body_force,
                self.viscosity,
                inv_density,
                time_step
            )
        );

        Self::correct_velocities_for_geometry(
            grid,
            &self.no_slip_corrections,
            &self.slip_mirror_stencils,
            &mut self.velocity_star
        );

        self.boundary_conditions.set_ghost_cells(grid, &mut self.velocity_star);
    }

    /// Computes the right hand side of the pressure Poisson equation from `velocity_star`, and
    /// writes it to `rhs`, which is stored on the **interior** grid.
    pub fn compute_pressure_rhs(
        &self,
        grid: &Grid,
        time_step: Float,
        rhs: &mut [Float]
    ) {
        let inv_time_step = 1.0 / time_step;

        rhs.par_iter_mut()
            .enumerate()
            .for_each(|(i_flat_interior, value)| {
                *value = pressure_rhs_kernel(
                    i_flat_interior,
                    grid,
                    &self.velocity_star,
                    self.density,
                    inv_time_step
                );
            });
    }

    pub fn update_velocity(
        &mut self,
        grid: &Grid,
        pressure: &[Float],
        time_step: Float
    ) {
        let inv_density = 1.0 / self.density;

        grid.parallel_spatial_vector_update(
            &mut self.velocity,
            |i, _current| add_pressure_gradient_kernel(
                i,
                grid,
                pressure,
                &self.velocity_star,
                inv_density,
                time_step
            )
        );

        Self::correct_velocities_for_geometry(
            grid,
            &self.no_slip_corrections,
            &self.slip_mirror_stencils,
            &mut self.velocity
        );

        self.boundary_conditions.set_ghost_cells(grid, &mut self.velocity);
    }

    /// Returns the largest magnitude of the (staggered) velocity vectors on the extended grid.
    /// Used to compute a time step from a Courant number.
    pub fn max_velocity(&self) -> Float {
        self.velocity.par_iter()
            .map(|velocity| velocity.length())
            .reduce(|| 0.0, Float::max)
    }

    /// Returns the cell-centered velocity at each of the cells in `cell_indices`, given as flat
    /// indices on the extended grid. Used to sample the velocity for the actuator line model
    /// without needing access to the full velocity field.
    pub fn cell_centered_velocity_at_cells(
        &self,
        grid: &Grid,
        cell_indices: &[usize]
    ) -> Vec<SpatialVector> {
        cell_indices.par_iter()
            .map(|&i_flat_extended| {
                let extended_indices = grid.extended_indices_from_flat_index(i_flat_extended);
                let interior_indices = grid.interior_indices_from_extended_indices(extended_indices);

                grid.cell_centered_value_from_face_staggered(interior_indices, &self.velocity)
            })
            .collect()
    }

    /// Sets the body force at each of the cells in `cell_indices`, given as flat indices on the
    /// extended grid. All other cells are left unchanged.
    pub fn set_body_force_at_cells(
        &mut self,
        cell_indices: &[usize],
        body_force: &[SpatialVector]
    ) {
        for (&i_flat_extended, &force) in cell_indices.iter().zip(body_force) {
            self.body_force[i_flat_extended] = force;
        }
    }

    /// Applies first the no-slip and then the slip geometry corrections to `velocity`
    pub fn correct_velocities_for_geometry(
        grid: &Grid,
        no_slip_corrections: &NoSlipCorrections,
        slip_mirror_stencils: &SlipMirrorStencils,
        velocity: &mut [SpatialVector]
    ) {
        Self::correct_velocities_for_no_slip_geometry(no_slip_corrections, velocity);
        Self::correct_velocities_for_slip_geometry(grid, slip_mirror_stencils, velocity);
    }

    /// No-slip correction, applied only to the precomputed list of faces near or inside a no-slip
    /// geometry (see `NoSlipCorrections`).
    pub fn correct_velocities_for_no_slip_geometry(
        corrections: &NoSlipCorrections,
        velocity: &mut [SpatialVector]
    ) {
        let velocity_ptr = velocity.as_mut_ptr() as usize;

        for axis_index in 0..3 {
            corrections.entries[axis_index].par_iter().for_each(|entry| {
                // Safe: within one `axis_index` pass every entry has a distinct `cell_index` by 
                // construction, and each entry only reads the component it writes, so all 
                // accesses this pass target disjoint `SpatialVector`s.
                unsafe {
                    let cell = &mut *(velocity_ptr as *mut SpatialVector).add(entry.cell_index);
                    cell[axis_index] = correct_no_slip_entry_kernel(entry, cell[axis_index]);
                }
            });
        }
    }

    /// Mirror/ghost-cell free-slip correction, applied only to the precomputed band of cells near
    /// a slip surface (see `SlipMirrorStencils`). Since the mirror construction samples the
    /// velocity field at an interpolated image point rather than only at the current cell, all the
    /// corrected components (for all three axes) are first computed from the uncorrected field
    /// into compact per-axis buffers, and only then written back. This makes the result
    /// independent of the order the entries are processed in, without copying the full field.
    pub fn correct_velocities_for_slip_geometry(
        grid: &Grid,
        stencils: &SlipMirrorStencils,
        velocity: &mut [SpatialVector]
    ) {
        let stride = grid.extended_stride;

        let new_components: [Vec<Float>; 3] = std::array::from_fn(|axis_index| {
            stencils.entries[axis_index].par_iter()
                .map(|entry| correct_slip_mirror_entry_kernel(entry, velocity, stride, axis_index))
                .collect()
        });

        let velocity_ptr = velocity.as_mut_ptr() as usize;

        for axis_index in 0..3 {
            stencils.entries[axis_index].par_iter()
                .zip(new_components[axis_index].par_iter())
                .for_each(|(entry, &new_component)| {
                    // Safe: within one `axis_index` pass every entry has a distinct `cell_index`
                    // by construction (at most one entry per cell per axis-list), so all writes
                    // this pass target disjoint `SpatialVector`s; passes themselves run
                    // sequentially.
                    unsafe {
                        let cell = &mut *(velocity_ptr as *mut SpatialVector).add(entry.cell_index);
                        cell[axis_index] = new_component;
                    }
                });
        }
    }
}
