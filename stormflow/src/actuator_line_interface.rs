use rayon::prelude::*;

use stormbird::actuator_line::ActuatorLine;
use stormath::type_aliases::Float;
use stormath::spatial_vector::SpatialVector;

use crate::grid::Grid;

/// Trilinear interpolation stencils of the cell-centered velocity at the wake points of the
/// lifting line correction. The stencils are rebuilt in each time step, as the points move.
pub struct WakePointStencils {
    /// The indices of the wake points that are inside the domain. Points outside the domain are
    /// not sampled, so that the model keeps the previous value.
    pub point_indices: Vec<usize>,
    /// Eight cells for each point, as flat indices on the extended grid
    pub cell_indices: Vec<usize>,
    /// The interpolation weight of each cell in `cell_indices`
    pub weights: Vec<Float>,
}

impl WakePointStencils {
    pub fn is_empty(&self) -> bool {
        self.point_indices.is_empty()
    }
}

pub struct ActuatorLineInterface {
    pub model: ActuatorLine,
    pub cell_indices_to_check: Vec<usize>,
    pub dominating_line_indices: Vec<usize>,
    pub dominating_line_weights: Vec<Float>,
}

impl ActuatorLineInterface {
    pub fn new(model: ActuatorLine, grid: &Grid) -> Self {
        let nr_interior_cells = grid.nr_interior_cells();

        let results: Vec<_> = (0..nr_interior_cells)
            .into_par_iter()
            .map(|i_flat_interior| {
                let interior_indices = grid.interior_indices_from_flat_index(i_flat_interior);
                let cell_center = grid.cell_center(interior_indices);

                let extended_indices = grid.extended_indices_from_interior_indices(interior_indices);
                let flat_index = grid.flat_index_on_extended_grid(extended_indices);
                
                let (line_index, projection_weight) = model.dominating_line_element_and_weight_at_point(cell_center);

                (flat_index, line_index, projection_weight)
            })
            .collect();

        let mut cell_indices_to_check = Vec::new();
        let mut dominating_line_indices = Vec::new();
        let mut dominating_line_weights = Vec::new();

        for i in 0..results.len() {
            let (flat_index, line_index, projection_weight) = results[i];

            // The limit is relative to the peak value of the weight of the line element
            let relative_weight = model.relative_projection_weight(line_index, projection_weight);

            if relative_weight > model.sampling_settings.weight_limit {
                cell_indices_to_check.push(flat_index);
                dominating_line_indices.push(line_index);
                dominating_line_weights.push(projection_weight);
            }
        }

        println!(
            "Number of cells with actuator line interaction: {:.?}", 
            cell_indices_to_check.len()
        );

        
        Self {
            model,
            cell_indices_to_check,
            dominating_line_indices,
            dominating_line_weights
        }
    }

    /// Updates the control point velocities of the model, based on `cell_velocities`, which must 
    /// hold the cell-centered velocity of each cell in `cell_indices_to_check`, in the same order.
    pub fn update_ctrl_points_velocity(
        &mut self, 
        grid: &Grid, 
        cell_velocities: &[SpatialVector]
    ) {           
        let nr_cells_to_check = self.cell_indices_to_check.len();
        
        let nr_span_lines = self.model.line_force_model.nr_span_lines();
        
        let mut numerator = vec![SpatialVector::default(); nr_span_lines];
        let mut denominator = vec![0.0; nr_span_lines];

        let cell_volume = grid.cell_length[0] * grid.cell_length[1] * grid.cell_length[2];
        
        for i in 0..nr_cells_to_check {
            let i_flat_extended = self.cell_indices_to_check[i];
            
            let extended_indices = grid.extended_indices_from_flat_index(i_flat_extended);
            let interior_indices = grid.interior_indices_from_extended_indices(extended_indices);
            
            let cell_center = grid.cell_center(interior_indices);
            
            let line_index = self.dominating_line_indices[i];
            
            let (temp_num, temp_den) = self.model.get_weighted_velocity_sampling_integral_terms_for_cell(
                line_index, 
                cell_velocities[i], 
                cell_center, 
                cell_volume
            );
            
            numerator[line_index] += temp_num;
            denominator[line_index] += temp_den;
        }
        
        for line_index in 0..nr_span_lines {
            if denominator[line_index] != 0.0 {
                self.model.ctrl_points_velocity[line_index] = numerator[line_index] / denominator[line_index];
            }
        }
    }

    /// Builds the stencils for interpolating the cell-centered velocity at the wake points of the
    /// lifting line correction. Empty unless the correction uses a dynamic wake. Points outside
    /// the domain are skipped. Points between the outermost cell centers and the boundary use the
    /// nearest cells.
    pub fn wake_point_stencils(&self, grid: &Grid) -> WakePointStencils {
        let wake_points = self.model.wake_points_to_sample();

        let mut stencils = WakePointStencils {
            point_indices: Vec::new(),
            cell_indices: Vec::with_capacity(8 * wake_points.len()),
            weights: Vec::with_capacity(8 * wake_points.len()),
        };

        let domain_start = grid.start_point;

        let domain_end = SpatialVector::from([
            grid.start_point[0] + grid.interior_shape[0] as Float * grid.cell_length[0],
            grid.start_point[1] + grid.interior_shape[1] as Float * grid.cell_length[1],
            grid.start_point[2] + grid.interior_shape[2] as Float * grid.cell_length[2],
        ]);

        // The cell centers of the interior grid
        let field_origin = grid.start_point + 0.5 * grid.cell_length;

        for (point_index, &point) in wake_points.iter().enumerate() {
            let is_inside = (0..3).all(|axis| {
                point[axis] >= domain_start[axis] && point[axis] <= domain_end[axis]
            });

            if !is_inside {
                continue;
            }

            // Points between the outermost cell centers and the boundary use the outermost cells
            let stencil = grid.trilinear_stencil_at_interior(field_origin, point);

            let base_indices = grid.interior_indices_from_flat_index(stencil.base_index);
            let weights = stencil.weights;

            stencils.point_indices.push(point_index);

            for a in 0..2 {
                for b in 0..2 {
                    for c in 0..2 {
                        stencils.cell_indices.push(
                            grid.flat_index_on_extended_grid_from_interior_indices([
                                base_indices[0] + a,
                                base_indices[1] + b,
                                base_indices[2] + c,
                            ])
                        );

                        stencils.weights.push(weights[0][a] * weights[1][b] * weights[2][c]);
                    }
                }
            }
        }

        stencils
    }

    /// Sets the sampled velocity at the wake points of the lifting line correction, from the
    /// cell-centered velocity at the cells in the `stencils`, in the same order.
    pub fn set_wake_points_velocity(
        &mut self,
        stencils: &WakePointStencils,
        cell_velocities: &[SpatialVector]
    ) {
        for (i, &point_index) in stencils.point_indices.iter().enumerate() {
            let velocity = (8 * i..8 * (i + 1))
                .map(|j| stencils.weights[j] * cell_velocities[j])
                .sum::<SpatialVector>();

            self.model.set_wake_point_velocity_at_index(point_index, velocity);
        }
    }

    /// Steps the model, based on `cell_velocities` (see `update_ctrl_points_velocity`)
    pub fn step_model(
        &mut self, 
        time: Float, 
        time_step: Float, 
        grid: &Grid, 
        cell_velocities: &[SpatialVector]
    ) {
        self.update_ctrl_points_velocity(grid, cell_velocities);

        self.model.do_step(time, time_step);
    }

    /// Returns the body force for each cell in `cell_indices_to_check`, in the same order, based 
    /// on `cell_velocities` (see `update_ctrl_points_velocity`).
    pub fn compute_body_force(
        &self, 
        cell_velocities: &[SpatialVector]
    ) -> Vec<SpatialVector> {
        let nr_cells_to_check = self.cell_indices_to_check.len();
        
        (0..nr_cells_to_check)
            .into_par_iter()
            .map(|i| {
                let cell_velocity = cell_velocities[i];
                
                let line_index = self.dominating_line_indices[i];
                
                let body_force_weight = self.dominating_line_weights[i];
                
                let line_force_force = self.model.force_to_project_at_cell(
                    line_index, 
                    cell_velocity
                );

                let spanwise_damping_force = self.model.spanwise_damping_flow(
                    line_index,
                    cell_velocity
                );

                let force_to_project = line_force_force + spanwise_damping_force;
            
                -(body_force_weight * force_to_project)
            }).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use stormbird::actuator_line::builder::ActuatorLineBuilder;
    use stormbird::actuator_line::corrections::lifting_line::{
        LiftingLineCorrectionBuilder,
        CorrectionWakeBuilder,
        DynamicCorrectionWakeBuilder,
    };
    use stormbird::line_force_model::builder::LineForceModelBuilder;
    use stormbird::line_force_model::builder::single_wing::WingBuilder;

    /// A linear velocity field, which the trilinear interpolation reproduces exactly
    fn linear_velocity(point: SpatialVector) -> SpatialVector {
        SpatialVector::from([1.0 + 0.1 * point[0], 0.2 * point[1], 0.05 * point[2]])
    }

    #[test]
    /// The wake points inside the domain get the interpolated cell-centered velocity, while the
    /// points outside the domain are skipped
    fn wake_points_get_the_interpolated_velocity() {
        let mut line_force_model = LineForceModelBuilder::new(6);

        line_force_model.add_wing(WingBuilder {
            section_points: vec![
                SpatialVector::from([0.0, 0.0, 1.0]),
                SpatialVector::from([0.0, 0.0, 4.0]),
            ],
            chord_vectors: vec![SpatialVector::from([1.0, 0.0, 0.0]); 2],
            ..Default::default()
        });

        let model = ActuatorLineBuilder {
            lifting_line_correction: Some(LiftingLineCorrectionBuilder {
                wake: CorrectionWakeBuilder::Dynamic(DynamicCorrectionWakeBuilder {
                    nr_panels_per_line_element: 20,
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..ActuatorLineBuilder::new(line_force_model)
        }.build().unwrap();

        // The wake is 0.75 + 18 * 0.5 + 1.0 = 10.75 chords long, which goes beyond the end of the
        // domain at x = 6
        let grid = Grid::new_direct(
            SpatialVector::from([-2.0, -2.0, 0.0]),
            SpatialVector::from([0.25, 0.25, 0.25]),
            [32, 16, 24]
        );

        let mut interface = ActuatorLineInterface::new(model, &grid);

        interface.model.ctrl_points_velocity = vec![
            SpatialVector::from([1.0, 0.0, 0.0]);
            interface.model.line_force_model.nr_span_lines()
        ];

        interface.model.do_step(0.0, 0.5);

        let stencils = interface.wake_point_stencils(&grid);
        let wake_points = interface.model.wake_points_to_sample().to_vec();

        assert!(!stencils.is_empty());
        assert!(stencils.point_indices.len() < wake_points.len());

        let cell_velocities: Vec<SpatialVector> = stencils.cell_indices.iter().map(|&flat_index| {
            let extended_indices = grid.extended_indices_from_flat_index(flat_index);
            let interior_indices = grid.interior_indices_from_extended_indices(extended_indices);

            linear_velocity(grid.cell_center(interior_indices))
        }).collect();

        // Values that are not overwritten
        let marker = SpatialVector::from([-1.0, -1.0, -1.0]);

        interface.model.set_wake_points_velocity(&vec![marker; wake_points.len()]);
        interface.set_wake_points_velocity(&stencils, &cell_velocities);

        let lifting_line_correction = interface.model.lifting_line_correction.as_ref().unwrap();

        let stormbird::actuator_line::corrections::lifting_line::CorrectionWake::Dynamic(wake) =
            &lifting_line_correction.wake else {
            panic!("Expected a dynamic wake");
        };

        for (point_index, point) in wake_points.iter().enumerate() {
            let sampled = wake.wake_points_velocity[point_index];

            if point[0] > 6.0 {
                assert_eq!(sampled, marker, "point {point_index} outside the domain");
            } else {
                let error = (sampled - linear_velocity(*point)).length();

                assert!(error < 1e-5, "point {point_index} at {point:?}: sampled {sampled:?}, exact {:?}", linear_velocity(*point));
            }
        }
    }
}
