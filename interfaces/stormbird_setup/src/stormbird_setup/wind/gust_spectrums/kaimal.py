"""
Copyright (C) 2024, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
"""

from dataclasses import dataclass

import numpy as np

from .gust_spectrum import GustSpectrum



@dataclass(kw_only = True, frozen = True, slots=True)
class Kaimal(GustSpectrum):
    height: float
    reference_velocity: float
    friction_velocity: float
    surface_roughness: float
    
    @property
    def standard_deviation(self) -> float:
        a_x = np.sqrt(4.5 - 0.856 * np.log(self.surface_roughness))
        
        return a_x * self.friction_velocity
        
    @property
    def length_value(self) -> float:
        return 300.0 * (self.height / 300.0)**(0.46 + 0.074 * np.log(self.surface_roughness))
        
    def spectrum_value(self, frequency: float) -> float:
        numerator = 6.868 * self.length_value / self.reference_velocity
        
        denominator = (1.0 + 10.32 * frequency * self.length_value / self.reference_velocity)**(5.0/3.0)
        
        return self.standard_deviation**2 * numerator / denominator
