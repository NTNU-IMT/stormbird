pub mod boundary_condisitions;
pub mod kernels;
pub mod slip_mirror_stencils;

use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

use rayon::prelude::*;

use crate::grid::Grid;
use boundary_condisitions::VelocityBoundaryConditions;
use slip_mirror_stencils::SlipMirrorStencils;

use kernels::{
    correct_velocities_for_geometry::{
        correct_velocities_for_no_slip_geometry_kernel,
        correct_slip_mirror_entry_kernel
    },
    convect_and_diffuse::convect_and_diffuse_kernel,
    add_pressure_gradient::add_pressure_gradient_kernel
};

pub struct VelocitySolver {
    pub velocity_org: Vec<SpatialVector>,
    pub velocity_star: Vec<SpatialVector>,
    pub velocity: Vec<SpatialVector>,
    pub body_force: Vec<SpatialVector>,
    pub signed_distance_function: Vec<Float>,
    pub signed_distance_function_slip: Vec<Float>,
    pub normals_slip_surfaces: Vec<SpatialVector>,
    pub slip_mirror_stencils: SlipMirrorStencils,
    pub boundary_conditions: VelocityBoundaryConditions,
    pub no_slip_epsilon: Float,
    pub viscosity: Float,
    pub density: Float,
}

impl VelocitySolver {
    pub fn initialize_after_build(&mut self, grid: &Grid) {
        Self::correct_velocities_for_no_slip_geometry(
            grid,
            &self.signed_distance_function,
            self.no_slip_epsilon,
            &mut self.velocity
        );

        Self::correct_velocities_for_slip_geometry(
            grid,
            &self.slip_mirror_stencils,
            &mut self.velocity
        );

        Self::correct_velocities_for_no_slip_geometry(
            grid,
            &self.signed_distance_function,
            self.no_slip_epsilon,
            &mut self.velocity_star
        );

        Self::correct_velocities_for_slip_geometry(
            grid,
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

        Self::correct_velocities_for_no_slip_geometry(
            grid,
            &self.signed_distance_function,
            self.no_slip_epsilon,
            &mut self.velocity_star
        );

        Self::correct_velocities_for_slip_geometry(
            grid,
            &self.slip_mirror_stencils,
            &mut self.velocity_star
        );

        self.boundary_conditions.set_ghost_cells(grid, &mut self.velocity_star);
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

        Self::correct_velocities_for_no_slip_geometry(
            grid,
            &self.signed_distance_function,
            self.no_slip_epsilon,
            &mut self.velocity
        );

        Self::correct_velocities_for_slip_geometry(
            grid,
            &self.slip_mirror_stencils,
            &mut self.velocity
        );

        self.boundary_conditions.set_ghost_cells(grid, &mut self.velocity);
    }

    pub fn correct_velocities_for_no_slip_geometry(
        grid: &Grid,
        signed_distance_function: &[Float],
        epsilon: Float,
        velocity: &mut [SpatialVector]
    ) {
        grid.parallel_spatial_vector_update(
            velocity,
            |i, current| correct_velocities_for_no_slip_geometry_kernel(
                i,
                grid,
                current,
                signed_distance_function,
                epsilon
            )
        );
    }

    /// Mirror/ghost-cell free-slip correction, applied only to the precomputed band of cells near
    /// a slip surface (see `SlipMirrorStencils`). Since the mirror construction samples the
    /// velocity field at an interpolated image point rather than only at the current cell, it
    /// needs a read-only snapshot of `velocity` to sample from while `velocity` itself is being
    /// written; that same snapshot is shared across all three axis passes below, so each
    /// component's correction is computed from the same pre-correction field regardless of pass
    /// order.
    pub fn correct_velocities_for_slip_geometry(
        grid: &Grid,
        stencils: &SlipMirrorStencils,
        velocity: &mut [SpatialVector]
    ) {
        let velocity_snapshot = velocity.to_vec();
        let stride = grid.extended_stride;
        let velocity_ptr = velocity.as_mut_ptr() as usize;

        for axis_index in 0..3 {
            stencils.entries[axis_index].par_iter().for_each(|entry| {
                let new_component = correct_slip_mirror_entry_kernel(
                    entry,
                    &velocity_snapshot,
                    stride,
                    axis_index
                );

                // Safe: within one `axis_index` pass every entry has a distinct `cell_index` by
                // construction (at most one entry per cell per axis-list), so all writes this
                // pass target disjoint `SpatialVector`s; passes themselves run sequentially.
                unsafe {
                    let cell = &mut *(velocity_ptr as *mut SpatialVector).add(entry.cell_index);
                    cell[axis_index] = new_component;
                }
            });
        }
    }
}
