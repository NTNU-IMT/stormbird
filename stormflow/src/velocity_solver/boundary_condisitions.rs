
use stormath::spatial_vector::SpatialVector;

use stormbird::wind::{
    environment::WindEnvironment,
    wind_condition::WindCondition,
};

use crate::grid::Grid;
use crate::grid::INTERIOR_OFFSET;
use crate::grid::boundary_face::BoundaryFace;

use rayon::prelude::*;

#[derive(Debug, Clone, Copy)]
pub enum VelocityBoundaryCondition {
    ZeroGradient,
    InletOutlet,
    SlipWall,
}

#[derive(Debug, Clone)]
pub struct VelocityBoundaryConditions {
    pub wind_environment: WindEnvironment,
    pub wind_condition: WindCondition,
    pub linear_velocity: SpatialVector,
    pub face_conditions: [[VelocityBoundaryCondition; 2]; 3]
}

impl VelocityBoundaryConditions {
    pub fn new(
        wind_environment: &WindEnvironment, 
        wind_condition: &WindCondition, 
        linear_velocity: SpatialVector, 
        up_direction: SpatialVector,
        slip_wall_boundary_override: [[bool; 2]; 3]
    ) -> Self {
        let mut face_conditions = [[VelocityBoundaryCondition::InletOutlet; 2]; 3];

        let mut up_axis: usize = 0;

        if up_direction[1].abs() > up_direction[0].abs()  && 
            up_direction[1].abs() > up_direction[2].abs() {
            up_axis = 1;
        } else if up_direction[2].abs() > up_direction[0].abs()  && 
            up_direction[2].abs() > up_direction[1].abs() {
            up_axis = 2;
        }

        face_conditions[up_axis][0] = VelocityBoundaryCondition::SlipWall;
        face_conditions[up_axis][1] = VelocityBoundaryCondition::ZeroGradient;

        // A bit of a hacky way to apply override the BC to slip walls if the user has set to so
        for axis_index in 0..3 {
            for face_index in 0..2 {
                if slip_wall_boundary_override[axis_index][face_index] {
                    face_conditions[axis_index][face_index] = VelocityBoundaryCondition::SlipWall;
                }
            }
        }

        Self {
            wind_environment: wind_environment.clone(),
            wind_condition: wind_condition.clone(),
            linear_velocity,
            face_conditions
        }
    }

    #[inline(always)]
    pub fn velocity_at_point(&self, point: SpatialVector) -> SpatialVector {
        self.wind_environment.steady_apparent_wind_velocity_vector_at_location(
            &self.wind_condition, point, self.linear_velocity
        )
    }

    pub fn initial_velocity(&self, grid: &Grid) -> Vec<SpatialVector> {
        let nr_extended_cells = grid.nr_extended_cells();

        (0..nr_extended_cells).into_iter().map(|i_flat_extended| {
            let extended_indices = grid.extended_indices_from_flat_index(i_flat_extended);

            let mut velocity = SpatialVector::default();
            let cell_center = grid.cell_center_extended(extended_indices); 

            for axis_index in 0..3 {
                let mut face_point = cell_center;
                face_point[axis_index] += 0.5 * grid.cell_length[axis_index]; // TODO: check if this is consistent over all the code...

                let face_velocity = self.velocity_at_point(face_point);

                velocity[axis_index] = face_velocity[axis_index]
            }

            velocity
        }).collect()
    }
    
    /// Updates the ghost cells on one face (`axis_index`/`face_index`), in parallel over the
    /// face's cells. Mirrors `PressureBoundaryConditions::set_ghost_cells_kernel`: a raw pointer
    /// lets the closure write `flat_current` while reads of `flat_neighbor` go through the
    /// ordinary slice, which is sound because `flat_current`/`flat_neighbor` are always on
    /// different layers along `axis_index` and this kernel only ever runs for one face at a time.
    fn set_ghost_cells_kernel(
        &self,
        axis_index: usize,
        condition: VelocityBoundaryCondition,
        boundary_face: &BoundaryFace,
        grid: &Grid,
        velocity: &mut [SpatialVector],
    ) {
        let velocity_ptr = velocity.as_mut_ptr() as usize;
        let [outer_len, inner_len] = boundary_face.shape;

        (0..outer_len * inner_len)
            .into_par_iter()
            .with_min_len(2048)
            .for_each(|idx| {
                let i_outer = idx / inner_len;
                let i_inner = idx % inner_len;

                let flat_current = (
                    boundary_face.axis_offset
                    + i_outer * boundary_face.stride[0]
                    + i_inner * boundary_face.stride[1]
                ) as usize;

                let flat_neighbor = (flat_current as i32 + boundary_face.neighbor_delta) as usize;

                let new_value = match condition {
                    VelocityBoundaryCondition::InletOutlet => {
                        // Check the direction of the domain flow at the cell center. 
                        // `neighbor_delta`is positive on the min-boundary face (neighbor is toward 
                        // +axis) and negative on the max-boundary face, so its sign alone tells us 
                        // which flow direction counts as inflow, without needing `face_index` here.
                        let at_the_min_boundary_face = boundary_face.neighbor_delta > 0;

                        //let neighbor_axis_flow = velocity[flat_neighbor][axis_index];

                        let extended_indices = grid.extended_indices_from_flat_index(flat_current);
                        let cell_center = grid.cell_center_extended(extended_indices);
                        let domain_flow = self.velocity_at_point(cell_center)[axis_index];

                        let inflow = if at_the_min_boundary_face{
                            domain_flow > 0.0
                        } else {
                            domain_flow < 0.0
                        };

                        if inflow {
                            let mut new_value = SpatialVector::default();
                            for c in 0..3 {
                                let mut face_point = cell_center;
                                face_point[c] += 0.5 * grid.cell_length[c]; // positive-face convention
                                new_value[c] = self.velocity_at_point(face_point)[c];
                            }
                            new_value
                        } else {
                            velocity[flat_neighbor]
                        }
                    },
                    VelocityBoundaryCondition::ZeroGradient => velocity[flat_neighbor],
                    VelocityBoundaryCondition::SlipWall => {
                        let mut v = velocity[flat_neighbor];
                        v[axis_index] = 0.0;
                        v
                    }
                };

                unsafe {
                    *(velocity_ptr as *mut SpatialVector).add(flat_current) = new_value;
                }
            });
    }

    /// Updates the ghost cells on the velocity, using the boundary conditions in self and the
    /// supplied grid for the indexing logic.
    pub fn set_ghost_cells(&self, grid: &Grid, velocity: &mut [SpatialVector]) {
        for axis_index in 0..3 {
            for face_index in 0..2 {
                let condition = self.face_conditions[axis_index][face_index];

                for ghost_layer in 0..INTERIOR_OFFSET {
                    let boundary_face = BoundaryFace::new(
                        grid.extended_shape,
                        grid.extended_stride,
                        axis_index,
                        face_index,
                        ghost_layer
                    );

                    self.set_ghost_cells_kernel(axis_index, condition, &boundary_face, grid, velocity);
                }
            }
        }
    }
}