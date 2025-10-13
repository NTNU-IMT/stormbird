

from .base_model import StormbirdSetupBaseModel
from .spatial_vector import SpatialVector
from .section_model import SectionModel

from pydantic import Field


class WingBuilder(StormbirdSetupBaseModel):
    '''
    Class for defining a wing model builder
    '''
    section_points: list[SpatialVector]
    chord_vectors: list[SpatialVector]
    section_model: SectionModel
    non_zero_circulation_at_ends: tuple[bool, bool] = (False, False)
    nr_sections: int | None = None

class LineForceModelBuilder(StormbirdSetupBaseModel):
    '''
    Interface to the line force model builder
    '''
    wing_builders: list[WingBuilder] = []
    nr_sections: int = 20
    density: float = 1.225
    local_wing_angles: list[float] = []
    rotation: SpatialVector = SpatialVector()
    translation: SpatialVector = SpatialVector()

    def add_wing_builder(self, wing_builder: WingBuilder):
        self.wing_builders.append(wing_builder)