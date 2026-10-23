"""
Copyright (C) 2024, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
"""

from .base_model import StormbirdSetupBaseModel
from .spatial_vector import SpatialVector
from .section_models import SectionModel, RotatingCylinder

from .circulation_corrections import CirculationCorrectionBuilder

from .input_power import InputPowerModel

from enum import Enum
from pydantic import model_validator
from typing import Iterator

class CoordinateSystem(Enum):
    Global = "Global"
    Body = "Body"

class ForceCorrection(StormbirdSetupBaseModel):
    '''
    Empirical corrections of the forces on a wing. All corrections are zero by default.

    The lift-induced drag correction adds a drag coefficient equal to the factor times the square
    of the lift coefficient on each line element, computed from the total lift force. A positive
    value increases the drag, while a negative value decreases it.
    '''
    lift_induced_drag_correction: float = 0.0

class AddedMass(StormbirdSetupBaseModel):
    '''
    Strip theory model of the added mass forces on a wing. The coefficients are relative to the
    added mass of a flat plate accelerating normal to its chord, density * pi * chord^2 / 4 per
    span length. Typical values are:

    - Thin foil: normal_coefficient = 1.0, chordwise_coefficient = 0.0
    - Thick foil: normal_coefficient = 1.0, chordwise_coefficient = (thickness / chord)^2
    - Circular cylinder (rotor sail): normal_coefficient = 1.0, chordwise_coefficient = 1.0

    Both coefficients are zero by default, which means that the model is not used.
    '''
    normal_coefficient: float = 0.0
    chordwise_coefficient: float = 0.0

class GyroscopicMoment(StormbirdSetupBaseModel):
    '''
    Model of the gyroscopic moment from a rotor sail. The moment of inertia is the total moment of
    inertia of the rotor about its axis of rotation, in kg m^2. Only used together with the
    rotating cylinder section model. Zero by default, which means that the model is not used.
    '''
    moment_of_inertia: float = 0.0

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
    internal_compartment_boundaries: list[float] | None = None
    force_correction: ForceCorrection | None = None
    added_mass: AddedMass | None = None
    gyroscopic_moment: GyroscopicMoment | None = None

    @model_validator(mode='after')
    def check_gyroscopic_moment(self) -> "WingBuilder":
        '''
        The gyroscopic moment uses the internal state of the wing as the rotational speed of the
        rotor, and is therefore only allowed together with the rotating cylinder section model.
        '''
        if (
            self.gyroscopic_moment is not None and
            self.gyroscopic_moment.moment_of_inertia != 0.0 and
            not isinstance(self.section_model.model, RotatingCylinder)
        ):
            raise ValueError(
                "The gyroscopic moment model is only used together with the rotating cylinder "
                "section model"
            )

        return self

    @model_validator(mode='after')
    def check_internal_compartments(self) -> "WingBuilder":
        '''
        The internal compartments divide the internal volume of the wing into sealed compartments,
        defined by the non-dimensional span locations of their boundaries, including the ends of
        the wing at -0.5 and 0.5. More than one compartment is only allowed for section models with
        an internal flow model.
        '''
        boundaries = self.internal_compartment_boundaries

        if boundaries is None:
            return self

        if len(boundaries) < 2:
            raise ValueError(
                "The internal compartment boundaries must at least contain the two ends of the wing"
            )

        if abs(boundaries[0] + 0.5) > 1e-12 or abs(boundaries[-1] - 0.5) > 1e-12:
            raise ValueError("The internal compartment boundaries must start at -0.5 and end at 0.5")

        if any(b <= a for a, b in zip(boundaries[:-1], boundaries[1:])):
            raise ValueError("The internal compartment boundaries must be strictly increasing")

        if len(boundaries) > 2 and not self.section_model.uses_internal_compartments():
            raise ValueError(
                "Internal compartments are only used by section models with an internal flow "
                "model, i.e., an active air flow foil with the PowerToInternalFlow or "
                "FanSpeedToInternalFlow conversion"
            )

        return self

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
