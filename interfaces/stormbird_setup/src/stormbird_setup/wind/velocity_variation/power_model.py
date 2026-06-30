"""
Copyright (C) 2024, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
"""

from ...base_model import StormbirdSetupBaseModel

import numpy as np


class PowerModel(StormbirdSetupBaseModel):
    reference_velocity: float
    reference_height: float = 10.0
    power_factor: float = 0.1111111111111

    @classmethod
    def new_from_velocity_at_heights(
        cls,
        reference_velocity: float,
        sample_height_velocity: float,
        sample_height: float,
        reference_height: float = 10.0
    ) -> "PowerModel":
        """
        Fit the power factor, alpha, using the following logic:

        u_sample = u_ref * (z_sample / z_ref)**alpha
        u_sample / u_ref = (z_sample / z_ref)**alpha
        log(u_sample/u_ref) = alpha * log(z_sample/z_ref)
        
        alpha = log(u_sample/u_ref) / log(z_sample/z_ref)
        """

        power_factor = (
            np.log(sample_height_velocity / reference_velocity) / 
            np.log(sample_height / reference_height)
        )

        return cls(
            reference_velocity = reference_velocity,
            reference_height = reference_height,
            power_factor = power_factor
        )

    def velocity_at_height(self, height: float) -> float:
        increase_factor = self.velocity_increase_factor(height)
        
        return self.reference_velocity * increase_factor

    def velocity_increase_factor(self, height: float) -> float:
        if self.power_factor > 0.0:
            return (height / self.reference_height)**self.power_factor
        else:
            return 1.0
