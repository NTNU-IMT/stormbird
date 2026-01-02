'''
Copyright (C) 2024, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
'''

from ..base_model import StormbirdSetupBaseModel

import math

class SpatialVector(StormbirdSetupBaseModel):
    '''
    Class for defining a spatial vector
    '''
    x: float = 0.0
    y: float = 0.0
    z: float = 0.0
    
    @classmethod
    def from_dict(cls, dict_in) -> "SpatialVector":
        return cls(x = dict_in["x"], y = dict_in["y"], z = dict_in["z"])
        
    @classmethod
    def from_list(cls, list_in: list[float]) -> "SpatialVector":
        return cls(x = list_in[0], y = list_in[1], z = list_in[2])
        
    def length(self) -> float:
        return math.sqrt(self.x**2 + self.y**2 + self.z**2)

    def as_tuple(self) -> tuple[float, float, float]:
        return (self.x, self.y, self.z)
    
    def as_list(self) -> list[float]:
        return [self.x, self.y, self.z]
