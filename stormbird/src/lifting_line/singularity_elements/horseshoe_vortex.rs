

use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

use super::vortex_line;

#[derive(Clone, Debug)]
pub struct HorseshoeVortex {
    pub bound_vortex: [SpatialVector; 2],
    pub start_trailing_vortex: [SpatialVector; 2],
    pub end_trailing_vortex: [SpatialVector; 2],
    pub viscous_core_length: Float,
}

impl HorseshoeVortex {
    pub fn induced_velocity_with_unit_strength(
        &self,
        ctrl_point: SpatialVector,
    ) -> SpatialVector {
        let mut induced_velocity = SpatialVector::default();

        // Induced velocity from bound vortex
        induced_velocity += vortex_line::induced_velocity_from_line_with_unit_strength(
            &self.bound_vortex,
            ctrl_point,
            self.viscous_core_length,
        );

        // Induced velocity from start trailing vortex
        induced_velocity += vortex_line::induced_velocity_from_line_with_unit_strength(
            &self.start_trailing_vortex,
            ctrl_point,
            self.viscous_core_length,
        );

        // Induced velocity from end trailing vortex
        induced_velocity += vortex_line::induced_velocity_from_line_with_unit_strength(
            &self.end_trailing_vortex,
            ctrl_point,
            self.viscous_core_length,
        );

        induced_velocity
    }
}
