"""
Copyright (C) 2024, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
"""

from .base_model import StormbirdSetupBaseModel
from .spatial_vector import SpatialVector
from .section_models import SectionModel

from .circulation_corrections import CirculationCorrectionBuilder

from .input_power import InputPowerModel

from enum import Enum
from typing import Iterator

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
    line_segment_is_virtual: list[bool] | None = None
    non_zero_circulation_at_ends: tuple[bool, bool] = (False, False)
    nr_sections: int | None = None
    input_power_model: InputPowerModel = InputPowerModel()

    def real_line_segments(self) -> Iterator[int]:
        '''
        The indices of the line segments that are not virtual. Line segment `i` goes from section
        point `i` to section point `i + 1`. Virtual segments represent things like end plates, and
        are not a part of the wing itself, so they are left out of the span and the projected area.
        '''
        for i in range(len(self.section_points) - 1):
            if self.line_segment_is_virtual is None or not self.line_segment_is_virtual[i]:
                yield i

    def span(self) -> float:
        '''
        The span of the wing, which is the length of the line that the section points define.
        Virtual line segments are not included.
        '''
        return sum(
            (self.section_points[i + 1] - self.section_points[i]).length()
            for i in self.real_line_segments()
        )

    def projected_area(self) -> float:
        '''
        The projected area of the wing, which is the chord length integrated over the span. Virtual
        line segments are not included. The chord length is assumed to vary linearly between two
        section points, so each line segment contributes its own length multiplied with the average
        of the chord lengths at its two ends.
        '''
        total = 0.0

        for i in self.real_line_segments():
            segment_length = (self.section_points[i + 1] - self.section_points[i]).length()
            mean_chord = 0.5 * (
                self.chord_vectors[i].length() + self.chord_vectors[i + 1].length()
            )

            total += segment_length * mean_chord

        return total
    
class ForceCalculationSettings(StormbirdSetupBaseModel):
    include_viscous_lift_in_the_circulation: bool = False

class LineForceModelBuilder(StormbirdSetupBaseModel):
    '''
    Interface to the line force model builder
    '''
    wing_builders: list[WingBuilder] = []
    nr_sections: int = 32
    density: float = 1.225
    local_wing_angles: list[float] = []
    rotation: SpatialVector = SpatialVector()
    translation: SpatialVector = SpatialVector()
    circulation_correction: CirculationCorrectionBuilder = CirculationCorrectionBuilder()
    output_coordinate_system: CoordinateSystem = CoordinateSystem.Global
    force_calculation_settings: ForceCalculationSettings = ForceCalculationSettings()

    def add_wing_builder(self, wing_builder: WingBuilder):
        self.wing_builders.append(wing_builder)
        self.local_wing_angles.append(0.0)

    def spans(self) -> list[float]:
        '''The span of each wing, see `WingBuilder.span`'''
        return [wing_builder.span() for wing_builder in self.wing_builders]

    def projected_areas(self) -> list[float]:
        '''The projected area of each wing, see `WingBuilder.projected_area`'''
        return [wing_builder.projected_area() for wing_builder in self.wing_builders]
