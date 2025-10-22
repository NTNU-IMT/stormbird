'''
Copyright (C) 2024, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
'''

from ..base_model import StormbirdSetupBaseModel
from .spatial_vector import SpatialVector
from .section_models import SectionModel

from .circulation_corrections import CirculationCorrectionBuilder

from .input_power import InputPowerModel

from enum import Enum

class CoordinateSystem(Enum):
    Global = "Global"
    Body = "Body"

class WingBuilder(StormbirdSetupBaseModel):
    '''
    Class for defining a wing model builder
    '''
    section_points: list[SpatialVector]
    chord_vectors: list[SpatialVector]
    section_model: SectionModel
    non_zero_circulation_at_ends: tuple[bool, bool] = (False, False)
    nr_sections: int | None = None
    input_power_model: InputPowerModel = InputPowerModel()

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
    circulation_correction: CirculationCorrectionBuilder = CirculationCorrectionBuilder()
    output_coordinate_system: CoordinateSystem = CoordinateSystem.Global

    def add_wing_builder(self, wing_builder: WingBuilder):
        self.wing_builders.append(wing_builder)
        self.local_wing_angles.append(0.0)
