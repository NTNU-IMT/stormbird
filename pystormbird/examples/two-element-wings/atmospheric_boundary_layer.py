'''
Mode to represent the atmospheric boundary layer.
'''

from dataclasses import dataclass

import numpy as np

from pystormbird import SpatialVector

@dataclass
class AtmosphericBoundaryLayer():
    ship_velocity: float
    reference_wind_velocity: float
    wind_direction: float
    reference_height: float = 10.0
    power_factor: float = 1.0 / 9.0

    def get_velocity(self, position: SpatialVector):
        height = position.z

        increase_factor = (height / self.reference_height) ** self.power_factor

        wind_velocity = self.reference_wind_velocity * increase_factor

        return SpatialVector(
            self.ship_velocity + wind_velocity * np.cos(self.wind_direction),
            wind_velocity * np.sin(self.wind_direction),
            0.0
        )
        