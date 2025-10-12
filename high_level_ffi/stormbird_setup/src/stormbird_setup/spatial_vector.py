

from .base_model import StormbirdSetupBaseModel

class SpatialVector(StormbirdSetupBaseModel):
    '''
    Class for defining a spatial vector
    '''
    x: float
    y: float
    z: float