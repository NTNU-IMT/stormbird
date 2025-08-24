use stormath::{
    type_aliases::Float,
    spatial_vector::SpatialVector,
};

#[cfg(feature = "padded_spatial_vectors")]
const NED_ZERO_DIRECTION: SpatialVector = SpatialVector{0:[-1.0, 0.0, 0.0, 0.0]};
#[cfg(feature = "padded_spatial_vectors")]
const NED_UP_DIRECTION: SpatialVector = SpatialVector{0:[0.0, 0.0, 1.0, 0.0]};

#[cfg(not(feature = "padded_spatial_vectors"))]
const NED_ZERO_DIRECTION: SpatialVector = SpatialVector{0:[-1.0, 0.0, 0.0]};
#[cfg(not(feature = "padded_spatial_vectors"))]
const NED_UP_DIRECTION: SpatialVector = SpatialVector{0:[0.0, 0.0, -1.0]};

#[derive(Debug, Clone, Copy)]
pub struct WindCondition {
    pub velocity: Float,
    pub direction_coming_from: Float,
}

impl WindCondition {
    pub fn from_velocity_vector_assuming_ned(
        velocity_vector: SpatialVector,
    ) -> Self {

        let direction_coming_from = velocity_vector.signed_angle_between(NED_ZERO_DIRECTION, NED_UP_DIRECTION);

        WindCondition {
            velocity: velocity_vector.length(),
            direction_coming_from
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_velocity_vector() {
        let wind_velocity = 8.2;
        let allowable_error = 1e-6;

        // x-axis points forward -> negative x component is wind from the north
        let north_wind_vector = SpatialVector::new(-wind_velocity, 0.0, 0.0);
        
        // y-axis points towards the east -> negative y component is wind from the east side
        let east_wind_vector = SpatialVector::new(0.0, -wind_velocity, 0.0);

        let south_wind_vector = SpatialVector::new(wind_velocity, 0.0, 0.0);

        let west_wind_vector = SpatialVector::new(0.0, wind_velocity, 0.0);

        let north_wind_condition = WindCondition::from_velocity_vector_assuming_ned(
            north_wind_vector
        );

        let east_wind_condition = WindCondition::from_velocity_vector_assuming_ned(
            east_wind_vector
        );

        let south_wind_condition = WindCondition::from_velocity_vector_assuming_ned(
            south_wind_vector
        );

        let west_wind_condition = WindCondition::from_velocity_vector_assuming_ned(
            west_wind_vector
        );

        assert!((north_wind_condition.velocity - wind_velocity).abs() < allowable_error);
        assert!((south_wind_condition.velocity - wind_velocity).abs() < allowable_error);
        assert!((east_wind_condition.velocity - wind_velocity).abs() < allowable_error);
        assert!((west_wind_condition.velocity - wind_velocity).abs() < allowable_error);

        assert!(north_wind_condition.direction_coming_from.abs() < allowable_error);
        assert!((south_wind_condition.direction_coming_from.to_degrees().abs() - 180.0).abs() < allowable_error);
        assert!((east_wind_condition.direction_coming_from.to_degrees() - 90.0).abs() < allowable_error);
        assert!((west_wind_condition.direction_coming_from.to_degrees() + 90.0).abs() < allowable_error);
    }

}