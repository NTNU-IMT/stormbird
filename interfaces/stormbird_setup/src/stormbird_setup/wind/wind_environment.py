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
    """
    The axis that the wind direction is rotated around. The default is opposite of the default up
    direction, so that the wind direction is measured clockwise seen from above. This is the same
    default as in Stormbird.
    """
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

    def linear_velocity_from_ship_velocity(self, ship_velocity: float) -> SpatialVector:
        '''
        Returns the linear velocity that is used for the apparent wind for a given ship velocity.
        The ship is assumed to sail opposite of the zero direction vector, so that the linear
        velocity is the ship velocity times the zero direction vector. This is the same definition
        as in the complete sail model in Stormbird.
        '''
        return self.zero_direction_vector * ship_velocity

    def apparent_wind_direction_from_condition_and_linear_velocity(
        self,
        condition: WindCondition,
        linear_velocity: SpatialVector,
        height: float = 10.0
    ) -> float:
        '''
        Returns the apparent wind direction at the given height, measured from the zero direction
        vector, around the wind rotation axis. This is the direction that is used to look up the
        inflow corrections. The default height of 10 m is the same as in the complete sail model in
        Stormbird.
        '''
        apparent_wind = self.steady_apparent_wind_velocity_vector_at_location(
            condition,
            self.up_direction * height,
            linear_velocity
        )

        return self.zero_direction_vector.signed_angle_between(
            apparent_wind,
            self.wind_rotation_axis
        )
