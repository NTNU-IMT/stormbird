"""
Copyright (C) 2024, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
"""

from dataclasses import dataclass

from .gust_spectrum import GustSpectrum

@dataclass(kw_only = True, frozen = True, slots=True)
class Froya(GustSpectrum):
    height: float
    reference_velocity: float
    power: float = 0.468
    
    @property
    def relative_height(self) -> float:
        return self.height / 10.0
        
    @property
    def relative_velocity(self) -> float:
        return self.reference_velocity / 10.0
    
    def spectrum_value(self, frequency: float) -> float:
        f_tilda = 172.0 * frequency * self.relative_height**(2/3.0) * self.relative_velocity**(-0.75)
        
        denominator = (1 + f_tilda**self.power)**(5.0 /(3.0 * self.power))
        
        numerator = self.relative_velocity**2 * self.relative_height**0.45
        
        return 320.0 * numerator / denominator
