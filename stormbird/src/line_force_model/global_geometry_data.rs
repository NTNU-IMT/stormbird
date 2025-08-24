
use super::*;

#[derive(Debug, Clone, Default)]
/// Structure for storing relevant global geometry data from the line force model. That is, this 
/// data structure stores the values with all transformations applied. This is to avoid repeated 
/// calculations of the same values.
pub struct GlobalLineForceModelGeometry {
    /// Ctrl points
    pub ctrl_points: Vec<SpatialVector>,
    /// The span lines of the line force model, with both translation and rotation applied.
    pub span_points: Vec<SpatialVector>,
    /// The chord vectors of the line force model, with both translation and rotation applied.
    pub chord_vectors_at_span_points: Vec<SpatialVector>,
}

impl GlobalLineForceModelGeometry {
    pub fn new(
        line_force_model: &LineForceModel,
    ) -> Self {
        let ctrl_points = line_force_model.ctrl_points();

        let span_points = line_force_model.span_points();
        let chord_vectors_at_ctrl_points = line_force_model.global_chord_vectors();

        let chord_vectors_at_span_points = line_force_model.span_point_values_from_ctrl_point_values(
            &chord_vectors_at_ctrl_points, false);

        Self {
            ctrl_points,
            span_points,
            chord_vectors_at_span_points
        }
    }
}
