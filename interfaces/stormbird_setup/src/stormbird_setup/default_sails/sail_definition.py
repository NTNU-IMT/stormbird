"""
Copyright (C) 2026, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
"""

import math

from pydantic import Field, model_validator

from ..base_model import StormbirdSetupBaseModel
from ..spatial_vector import SpatialVector
from ..section_models import SectionModel, RotatingCylinder
from ..line_force_model import WingBuilder, ForceCorrection, AddedMass
from ..controller import ControllerSetPoints
from ..circulation_corrections import CirculationCorrectionBuilder
from ..lifting_line.solver import Linearized, Iterative

# Margin between the angle of attack where the 2D lift of the foils in the section models is at its
# maximum, and the maximum angle of attack set point in the controller, used by all the default
# sails, in radians. The angles of maximum 2D lift are computed with the lift_coefficient method of
# the Foil class in pystormbird, and are stored with each default sail, as this library does not
# depend on pystormbird.
STALL_ANGLE_MARGIN = math.radians(5.0)

class SailDefinition(StormbirdSetupBaseModel):
    '''
    Definition of a sail that contains everything needed to use the sail in a complete sail model:
    the section model, the controller set points, and suitable simulation settings. The definition
    does not contain the size of the sail. The chord length (or diameter) and height are instead
    given when the definition is used, so that the same definition can be scaled to different sail
    sizes. All values are therefore either non-dimensional, or scaled with the chord length.

    Models that are tuned to data should be used with the same simulation settings as in the
    tuning, which are therefore part of the definition.

    The input power is not part of the definition, as it is computed by the section model.
    '''
    section_model: SectionModel
    controller_set_points: ControllerSetPoints
    solver: Linearized | Iterative = Field(default_factory=Linearized)
    """
    Solver settings that are suitable for the sail. For instance, rotor sails use a limit on the
    induced velocities.
    """
    circulation_correction: CirculationCorrectionBuilder = Field(
        default_factory=CirculationCorrectionBuilder
    )
    """Correction of the circulation distribution, such as smoothing. No correction by default."""
    virtual_extension_factors: tuple[float, float] = (0.0, 0.0)
    """
    Length of a virtual line segment below the bottom and above the top of the sail, relative to the
    chord length. Virtual segments carry circulation, but are not part of the sail itself, so they
    give no forces. They move the tip vortices away from the ends of the sail, which can be used to
    model end plates. No extensions by default.
    """
    max_tip_speed: float | None = None
    """
    Maximum velocity of the surface of a rotor sail, in m/s. Limits the revolutions per second,
    which is the wing internal state, so that the limit scales with the diameter. Only used together
    with the rotating cylinder section model. No limit by default.
    """
    force_correction: ForceCorrection | None = None
    added_mass: AddedMass | None = None

    @model_validator(mode='after')
    def check_parameters(self) -> "SailDefinition":
        if any(factor < 0.0 for factor in self.virtual_extension_factors):
            raise ValueError(
                f"The virtual extension factors can not be negative. Values: "
                f"{self.virtual_extension_factors}"
            )

        if self.max_tip_speed is not None and not isinstance(self.section_model.model, RotatingCylinder):
            raise ValueError("max_tip_speed is only used together with the rotating cylinder section model")

        if self.max_tip_speed is not None and self.max_tip_speed <= 0.0:
            raise ValueError(f"max_tip_speed must be larger than zero. Value: {self.max_tip_speed}")

        return self

    def wing_builder(
        self,
        *,
        position: SpatialVector,
        chord_length: float,
        height: float,
        chord_direction: SpatialVector = SpatialVector(x=1.0),
        up_direction: SpatialVector = SpatialVector(z=1.0),
        base_on_symmetry_plane: bool = False
    ) -> WingBuilder:
        '''
        Wing builder for a sail with the given size, standing vertically with its bottom at the given
        position. The span goes along the up direction, while the chord goes along the chord
        direction, which must be perpendicular to the up direction.

        The base of the sail should be marked as standing on the symmetry plane when the bottom is
        on the plane, as the sail then continues as a mirror image below it. The circulation is
        then not zero at the base, which is used by the circulation corrections, such as smoothing.
        For a sail standing above the plane, e.g., on the deck of a ship, the circulation is zero at
        both ends. A virtual extension below the base is not allowed for a sail on the plane.
        '''
        span_direction = up_direction.normalize()
        chord_direction_normalized = chord_direction.normalize()

        if abs(span_direction.dot(chord_direction_normalized)) > 1e-6:
            raise ValueError("The chord direction must be perpendicular to the up direction")

        chord_length = float(chord_length)
        height = float(height)

        bottom_point = position
        top_point = position + span_direction * height

        section_points = [bottom_point, top_point]
        line_segment_is_virtual = [False]

        bottom_extension_factor, top_extension_factor = self.virtual_extension_factors

        if base_on_symmetry_plane and bottom_extension_factor > 0.0:
            raise ValueError(
                "A sail with the base on the symmetry plane can not have a virtual extension below "
                "the base"
            )

        if bottom_extension_factor > 0.0:
            section_points.insert(0, bottom_point - span_direction * (bottom_extension_factor * chord_length))
            line_segment_is_virtual.insert(0, True)

        if top_extension_factor > 0.0:
            section_points.append(top_point + span_direction * (top_extension_factor * chord_length))
            line_segment_is_virtual.append(True)

        chord_vector = chord_direction_normalized * chord_length

        return WingBuilder(
            section_points = section_points,
            chord_vectors = [chord_vector.model_copy() for _ in section_points],
            section_model = self.section_model.model_copy(deep=True),
            line_segment_is_virtual = line_segment_is_virtual if any(line_segment_is_virtual) else None,
            non_zero_circulation_at_ends = (base_on_symmetry_plane, False),
            force_correction = self.force_correction,
            added_mass = self.added_mass
        )

    def scaled_controller_set_points(self, chord_length: float) -> ControllerSetPoints:
        '''
        Controller set points for a sail with the given chord length. Limits that depend on the size
        of the sail, such as the maximum revolutions per second of a rotor sail, are computed from
        the chord length.
        '''
        set_points = self.controller_set_points.model_copy(deep=True)

        if self.max_tip_speed is not None:
            set_points.wing_internal_state_max_value = self.max_tip_speed / (math.pi * chord_length)

        return set_points
