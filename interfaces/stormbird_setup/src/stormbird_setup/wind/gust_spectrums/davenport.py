
from dataclasses import dataclass

from .gust_spectrum import GustSpectrum

import numpy as np

@dataclass(kw_only = True, frozen = True, slots=True)
class Davenport(GustSpectrum):
    reference_velocity: float
    friction_velocity: float
    surface_roughness: float
    
    @property
    def standard_deviation(self) -> float:
        a_x = np.sqrt(4.5 - 0.856 * np.log(self.surface_roughness))
        
        return a_x * self.friction_velocity
        
    def spectrum_value(self, frequency: float) -> float:
        l_u = 1200.0
        
        numerator = (2.0/3.0) * (l_u/self.reference_velocity)**2 * frequency
        
        denominator = (1.0 + (frequency * l_u / self.reference_velocity)**2)**(4.0/3.0)
        
        return self.standard_deviation**2 * numerator / denominator
