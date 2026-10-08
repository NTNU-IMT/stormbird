use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

use rayon::prelude::*;

use crate::grid::Grid;

use super::VelocitySolverSetup;
use super::slip_mirror_stencils::SlipMirrorStencils;
use super::no_slip_corrections::NoSlipCorrections;
use super::wall_model::wall_stress_kernel;

use super::kernels::{
    correct_velocities_for_geometry::{
        correct_no_slip_entry_kernel,
        correct_slip_mirror_entry_kernel
    },
    convect_and_diffuse::convect_and_diffuse_kernel,
    turbulent_stress::convect_and_diffuse_turbulent_kernel,
    add_pressure_gradient::add_pressure_gradient_kernel,
    pressure_rhs::pressure_rhs_kernel
};

/// Clips each component of `velocity` to the range `[-limit, limit]`, and returns the number of
/// clipped components. The ghost cells are included, but they are always within the limit when
/// this is called, as they are set from the interior cells or the inlet velocity.
fn limit_velocity(velocity: &mut [SpatialVector], limit: Float) -> usize {
    velocity.par_iter_mut()
        .map(|value| {
            let mut nr_limited = 0;

            for component in 0..3 {
                if value[component] > limit {
                    value[component] = limit;
                    nr_limited += 1;
                } else if value[component] < -limit {
                    value[component] = -limit;
                    nr_limited += 1;
                }
            }

            nr_limited
        })
        .sum()
}

/// Velocity solver executed on the CPU, with all fields stored on the host.
pub struct VelocitySolverCPU {
    pub setup: VelocitySolverSetup,
    pub velocity_org: Vec<SpatialVector>,
    pub velocity_star: Vec<SpatialVector>,
    pub velocity: Vec<SpatialVector>,
    pub body_force: Vec<SpatialVector>,
    /// The cell-centered eddy viscosity, which is added to the momentum equation as a turbulent
    /// stress. `None` when no turbulence model is used, in which case the solver is unchanged. The
    /// field is set by the turbulence solver.
    pub eddy_viscosity: Option<Vec<Float>>,
    /// The number of velocity components clipped by the velocity limiter during the last time
    /// step, summed over all the inner iterations
    pub nr_limited_velocity_values: usize,
}

impl VelocitySolverCPU {
    /// Creates the solver. If `use_eddy_viscosity` is true, an eddy viscosity field is allocated,
    /// initialized to zero.
    pub fn new(
        setup: VelocitySolverSetup,
        initial_velocity: Vec<SpatialVector>,
        use_eddy_viscosity: bool
    ) -> Self {
        let nr_extended_cells = initial_velocity.len();

        Self {
            setup,
            velocity_org: initial_velocity.clone(),
            velocity_star: initial_velocity.clone(),
            velocity: initial_velocity,
            body_force: vec![SpatialVector::default(); nr_extended_cells],
            eddy_viscosity: use_eddy_viscosity.then(|| vec![0.0; nr_extended_cells]),
            nr_limited_velocity_values: 0,
        }
    }

    pub fn initialize_after_build(&mut self, grid: &Grid) {
        Self::correct_velocities_for_geometry(grid, &self.setup, &mut self.velocity);
        Self::correct_velocities_for_geometry(grid, &self.setup, &mut self.velocity_star);
    }

    pub fn initialize_before_step(&mut self, grid: &Grid) {
        self.setup.boundary_conditions.set_ghost_cells(
            grid,
            &mut self.velocity
        );

        self.velocity_org.copy_from_slice(&self.velocity);

        self.nr_limited_velocity_values = 0;
    }

    pub fn update_velocity_star(
        &mut self,
        grid: &Grid,
        time_step: Float
    ) {
        let inv_density = 1.0 / self.setup.density;

        match &self.eddy_viscosity {
            None => grid.parallel_spatial_vector_update(
                &mut self.velocity_star,
                |i, _current| convect_and_diffuse_kernel(
                    i,
                    grid,
                    &self.velocity_org,
                    &self.velocity,
                    &self.body_force,
                    self.setup.viscosity,
                    inv_density,
                    time_step
                )
            ),
            Some(eddy_viscosity) => grid.parallel_spatial_vector_update(
                &mut self.velocity_star,
                |i, _current| convect_and_diffuse_turbulent_kernel(
                    i,
                    grid,
                    &self.velocity_org,
                    &self.velocity,
                    &self.body_force,
                    eddy_viscosity,
                    self.setup.viscosity,
                    inv_density,
                    time_step
                )
            ),
        }

        self.apply_wall_stress(grid, time_step);

        if let Some(limit) = self.setup.velocity_limit {
            self.nr_limited_velocity_values += limit_velocity(&mut self.velocity_star, limit);
        }

        Self::correct_velocities_for_geometry(grid, &self.setup, &mut self.velocity_star);

        self.setup.boundary_conditions.set_ghost_cells(grid, &mut self.velocity_star);
    }

    /// Applies the wall shear stress of the wall model to `velocity_star`, with the reference
    /// velocity sampled from `velocity`. Does nothing when the no-slip geometries use the data
    /// immersion.
    fn apply_wall_stress(&mut self, grid: &Grid, time_step: Float) {
        let wall_stress = &self.setup.wall_stress;

        if wall_stress.entries.is_empty() {
            return;
        }

        let y_plus_lam = wall_stress.constants.y_plus_lam();
        let velocity = &self.velocity;
        let velocity_star = &self.velocity_star;

        // Several entries can belong to the same cell (for different axes), so all values are
        // computed before any is written
        let new_values: Vec<Float> = wall_stress.entries.par_iter().map(|entry| {
            wall_stress_kernel(
                entry,
                wall_stress.reference_distance,
                &wall_stress.constants,
                y_plus_lam,
                velocity,
                grid.extended_stride,
                self.setup.viscosity,
                time_step,
                velocity_star[entry.cell_index][entry.axis]
            )
        }).collect();

        for (entry, value) in wall_stress.entries.iter().zip(new_values) {
            self.velocity_star[entry.cell_index][entry.axis] = value;
        }
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
                    self.setup.density,
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
        let inv_density = 1.0 / self.setup.density;

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

        if let Some(limit) = self.setup.velocity_limit {
            self.nr_limited_velocity_values += limit_velocity(&mut self.velocity, limit);
        }

        Self::correct_velocities_for_geometry(grid, &self.setup, &mut self.velocity);

        self.setup.boundary_conditions.set_ghost_cells(grid, &mut self.velocity);
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
        setup: &VelocitySolverSetup,
        velocity: &mut [SpatialVector]
    ) {
        Self::correct_velocities_for_no_slip_geometry(&setup.no_slip_corrections, velocity);
        Self::correct_velocities_for_slip_geometry(grid, &setup.slip_mirror_stencils, velocity);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limit_velocity_clips_each_component() {
        let mut velocity = vec![
            SpatialVector([1.0, -2.0, 3.0]),
            SpatialVector([-5.0, 0.5, 2.5]),
        ];

        let nr_limited = limit_velocity(&mut velocity, 2.5);

        assert_eq!(nr_limited, 2);
        assert_eq!(velocity[0], SpatialVector([1.0, -2.0, 2.5]));
        assert_eq!(velocity[1], SpatialVector([-2.5, 0.5, 2.5]));
    }
}
