use super::*;

#[derive(Debug, Clone, Default)]
pub struct LineForceModelData {
    pub chord_vectors: Vec<SpatialVector<3>>,
    pub ctrl_points_velocity: Vec<SpatialVector<3>>,
    pub angles_of_attack: Vec<f64>,
    pub amount_of_flow_separation: Vec<f64>,
}

impl LineForceModelData {
    pub fn new(line_force_model: &LineForceModel) -> Self {
        Self {
            chord_vectors: line_force_model.chord_vectors(),
            ctrl_points_velocity: vec![SpatialVector::<3>::default(); line_force_model.ctrl_points().len()],
            angles_of_attack: vec![0.0; line_force_model.ctrl_points().len()],
            amount_of_flow_separation: vec![0.0; line_force_model.ctrl_points().len()],
        }
    }
}

impl Wake {
    pub fn update_line_force_model_data(
        &mut self, 
        line_force_model: &LineForceModel, 
        ctrl_points_freestream: &[SpatialVector<3>]
    ) {
        // Extract relevant information from the line force model
        let chord_vectors = line_force_model.chord_vectors();
        let ctrl_points = line_force_model.ctrl_points();

        // Compute the induced velocities at the control points
        let induced_velocities = if self.settings.end_index_induced_velocities_on_wake > 0 {
            self.induced_velocities(&ctrl_points)
        } else {
            vec![SpatialVector::<3>::default(); ctrl_points.len()]
        };

        let ctrl_points_velocity: Vec<SpatialVector<3>> = ctrl_points_freestream.iter().zip(induced_velocities.iter()).map(
            |(u_inf, u_i)| *u_inf + *u_i
        ).collect();

        let angles_of_attack = line_force_model.angles_of_attack(&ctrl_points_velocity);


        let mut amount_of_flow_separation = vec![0.0; ctrl_points.len()];
        for i in 0..self.indices.nr_panels_along_span {
            let wing_index = self.wing_index(i);

            amount_of_flow_separation[i] = line_force_model
                .section_models[wing_index]
                .amount_of_flow_separation(angles_of_attack[i]);
        }

        self.line_force_model_data = LineForceModelData {
            chord_vectors,
            ctrl_points_velocity,
            angles_of_attack,
            amount_of_flow_separation,
        };
    }
}