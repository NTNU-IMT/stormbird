"""
Copyright (C) 2024, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
"""

from dataclasses import dataclass

import numpy as np

from .gust_spectrum import GustSpectrum

@dataclass(kw_only = True, frozen = True, slots=True)
class OchiShin(GustSpectrum):
    height: float
    reference_velocity: float
    friction_velocity: float
    surface_roughness: float
    
    @property
    def standard_deviation(self) -> float:
        a_x = np.sqrt(4.5 - 0.856 * np.log(self.surface_roughness))
        
        return a_x * self.friction_velocity
        
    def spectrum_value(self, frequency: float) -> float:
        f_star = frequency * self.height / self.reference_velocity
        
        if f_star >= 0.0 and f_star <= 0.003:
            value = 583 * f_star
        elif f_star > 0.003 and f_star <= 0.1:
            value = 420 * f_star**0.7 / (1 + f_star**0.35)**11.5
        elif f_star > 0.1:
            value = 838 * f_star / (1 + f_star**0.35)**11.5
        else:
            value = 0.0
            
        return self.friction_velocity**2 * value / frequency
