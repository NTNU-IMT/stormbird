"""
Copyright (C) 2024, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
"""

from pydantic import Field

from ..base_model import StormbirdSetupBaseModel

from ..spatial_vector import SpatialVector

from .inflow_corrections import InflowCorrections
from .wind_condition import WindCondition

class WindEnvironment(StormbirdSetupBaseModel):
    up_direction: SpatialVector = Field(
        default_factory=lambda: SpatialVector(x=0.0, y=0.0, z=1.0)
    )
    wind_rotation_axis: SpatialVector = Field(
        default_factory=lambda: SpatialVector(x=0.0, y=0.0, z=-1.0)
    )
    zero_direction_vector: SpatialVector = Field(
        default_factory=lambda: SpatialVector(x=1.0, y=0.0, z=0.0)
    )
    water_plane_height: float = 0.0
    inflow_corrections: InflowCorrections | None = None

    def height_from_location(self, location: SpatialVector) -> float:
        '''
        Returns the height of a location above the water plane
        '''
        return max(location.dot(self.up_direction) - self.water_plane_height, 0.0)

    def true_wind_direction_vector(self, direction_coming_from: float) -> SpatialVector:
        '''
        Returns the true wind direction as a vector
        '''
        return self.zero_direction_vector.rotate_around_axis(
            direction_coming_from,
            self.wind_rotation_axis
        )

    def steady_true_wind_velocity_vector_at_location(
        self,
        condition: WindCondition,
        location: SpatialVector
    ) -> SpatialVector:
        '''
        Returns the steady true wind vector at the given location
        '''
        velocity = condition.steady_true_wind_velocity_at_height(self.height_from_location(location))

        return self.true_wind_direction_vector(condition.direction_coming_from) * velocity

    def steady_apparent_wind_velocity_vector_at_location(
        self,
        condition: WindCondition,
        location: SpatialVector,
        linear_velocity: SpatialVector
    ) -> SpatialVector:
        '''
        Returns the steady apparent wind vector at the given location
        '''
        return self.steady_true_wind_velocity_vector_at_location(condition, location) + linear_velocity
