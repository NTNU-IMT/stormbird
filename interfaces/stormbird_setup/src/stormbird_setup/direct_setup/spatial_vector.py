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
        
    def as_list(self) -> list[float]:
        return [self.x, self.y, self.z]
        
    def as_tuple(self) -> tuple[float, float, float]:
        return (self.x, self.y, self.z)
        
    def length(self) -> float:
        return math.sqrt(self.x**2 + self.y**2 + self.z**2)
        
    def length_squared(self) -> float:
        return self.x**2 + self.y**2 + self.z**2
        
    def dot(self, rhs: "SpatialVector") -> float:
        return self.x * rhs.x + self.y * rhs.y + self.z * rhs.z
        
    def cross(self, rhs: "SpatialVector") -> "SpatialVector":
        x = self.y * rhs.z - self.z * rhs.y
        y = self.z * rhs.x - self.x * rhs.z
        z = self.x * rhs.y - self.y * rhs.x
        
        return SpatialVector(x = x, y = y, z = z)
        
    def absolute_angle_between(self, rhs: "SpatialVector") -> float:
        self_len_sq = self.length_squared()
        rhs_len_sq = rhs.length_squared()

        if self_len_sq == 0.0 or rhs_len_sq == 0.0:
            return 0.0

        cosine_value = self.dot(rhs) / math.sqrt(self_len_sq * rhs_len_sq)

        clipped_cosine_value = min(max(cosine_value, (-1.0)), 1.0)

        return math.acos(clipped_cosine_value)
        
    def signed_angle_between(self, rhs: "SpatialVector", axis: "SpatialVector") -> float:
        triple_product = self.dot(rhs.cross(axis))

        absolute_angle = self.absolute_angle_between(rhs)

        if triple_product > 0.0:
            return absolute_angle
        else:
            return -absolute_angle

    
    
    
