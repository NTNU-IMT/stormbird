'''
Copyright (C) 2024, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
'''

from .base_model import StormbirdSetupBaseModel

class SpatialVector(StormbirdSetupBaseModel):
    '''
    Class for defining a spatial vector
    '''
    x: float = 0.0
    y: float = 0.0
    z: float = 0.0