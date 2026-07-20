pub mod boundary_condisitions;
pub mod parallel_interface;
pub mod kernels;

use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

use crate::grid::Grid;
use boundary_condisitions::VelocityBoundaryConditions;

use parallel_interface::parallel_spatial_vector_update;

use kernels::{
    correct_velocities_for_geometry::{
        correct_velocities_for_no_slip_geometry_kernel,
        correct_velocities_for_slip_geometry_kernel
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
    pub boundary_conditions: VelocityBoundaryConditions,
    pub no_slip_epsilon: Float,
    pub slip_epsilon: Float,
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
            &self.signed_distance_function_slip, 
            &self.normals_slip_surfaces,
            self.slip_epsilon, 
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
            &self.signed_distance_function_slip, 
            &self.normals_slip_surfaces,
            self.slip_epsilon, 
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
        
        parallel_spatial_vector_update(
            &mut self.velocity_star,
            grid,
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
            &self.signed_distance_function_slip, 
            &self.normals_slip_surfaces,
            self.slip_epsilon, 
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
        
        parallel_spatial_vector_update(
            &mut self.velocity, 
            grid, 
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
            &self.signed_distance_function_slip, 
            &self.normals_slip_surfaces,
            self.slip_epsilon, 
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
        parallel_spatial_vector_update(
            velocity, 
            grid, 
            |i, current| correct_velocities_for_no_slip_geometry_kernel(
                i, 
                grid, 
                current, 
                signed_distance_function, 
                epsilon
            )
        );
    }

    pub fn correct_velocities_for_slip_geometry(
        grid: &Grid,
        signed_distance_function: &[Float],
        normals: &[SpatialVector],
        epsilon: Float,
        velocity: &mut [SpatialVector]
    ) {
        parallel_spatial_vector_update(
            velocity, 
            grid, 
            |i, current| correct_velocities_for_slip_geometry_kernel(
                i, 
                grid, 
                current, 
                signed_distance_function, 
                normals,
                epsilon
            )
        );
    }
}