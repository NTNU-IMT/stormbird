

from .base_model import StormbirdSetupBaseModel
from .spatial_vector import SpatialVector

class WingBuilder(StormbirdSetupBaseModel):
    '''
    Class for defining a wing model builder
    '''
    section_points: list[SpatialVector]
    chord_vectors: list[SpatialVector]
    section_model: dict
    non_zero_circulation_at_ends: tuple[bool, bool] = (False, False)
    nr_sections: int | None = None

class LineForceModelBuilder(StormbirdSetupBaseModel):
    '''
    Interface to the line force model builder
    '''
    wing_builders: list[WingBuilder]
    nr_sections: int = 20
    density: float = 1.225
    local_wing_angles: list[float] = []
    rotation: SpatialVector = SpatialVector(x=0.0, y=0.0, z=0.0)
    translation: SpatialVector = SpatialVector(x=0.0, y=0.0, z=0.0)