use stormath::spatial_vector::SpatialVector;

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Wind {
    pub velocity: f64,
    pub direction_coming_from: f64,
}

impl Wind {
    /// Converts to a spatial vector in the maneuvering coordinate system.
    /// 
    /// Assumption:
    /// - The coordinate system has the x axis pointing towards the bow of the ship, the y axis
    ///  pointing to starboard, and the z axis pointing down.
    /// - The wind direction is measured as positive with a right hand rotation around the z axis.
    ///  This means that a wind coming from the bow of the ship has a direction of 0.0, and a wind
    ///  coming from starboard has a direction of PI/2.
    pub fn to_spatial_vector_maneuvering_coordinate_system(&self) -> SpatialVector<3> {
        let x = -self.velocity * self.direction_coming_from.cos();
        let y = -self.velocity * self.direction_coming_from.sin();

        SpatialVector([x, y, 0.0])
    }

    pub fn to_apparent_wind(&self, ship_velocity: f64) -> Wind {
        let x = -self.velocity * self.direction_coming_from.cos() - ship_velocity;
        let y = -self.velocity * self.direction_coming_from.sin();

        let direction_coming_from = -y.atan2(-x);
        let velocity = (x.powi(2) + y.powi(2)).sqrt();

        Wind {velocity, direction_coming_from}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apparent_wind_direction() {
        let angles_to_test_deg: Vec<f64> = vec![-90.0, -45.0, 0.0, 45.0, 90.0];

        let ship_velocity = 14.0 * 0.5144444;

        for angle in angles_to_test_deg {
            let wind = Wind {
                velocity: 10.0,
                direction_coming_from: angle.to_radians(),
            };

            let apparent_wind_zero_velocity = wind.to_apparent_wind(0.0);
            let apparent_wind_ship_velocity = wind.to_apparent_wind(ship_velocity);

            assert_eq!(apparent_wind_zero_velocity.direction_coming_from, wind.direction_coming_from);

            if angle > 0.0 {
                assert!(apparent_wind_ship_velocity.direction_coming_from < wind.direction_coming_from);
            }

            if angle < 0.0 {
                assert!(apparent_wind_ship_velocity.direction_coming_from > wind.direction_coming_from);
            }
            
        }
    }
}