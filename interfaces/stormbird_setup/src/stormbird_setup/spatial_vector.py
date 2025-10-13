

from .base_model import StormbirdSetupBaseModel

class SpatialVector(StormbirdSetupBaseModel):
    '''
    Class for defining a spatial vector
    '''
    x: float = 0.0
    y: float = 0.0
    z: float = 0.0