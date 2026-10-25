"""
Copyright (C) 2024, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
"""

from typing import TYPE_CHECKING

from pydantic import Field

from ..base_model import StormbirdSetupBaseModel
from ..spatial_vector import SpatialVector
from ..line_force_model import LineForceModelBuilder
from .simulation_builder import SimulationBuilder, QuasiSteadySettings
from .wake import QuasiSteadyWakeSettings, SymmetryCondition
from ..wind import WindEnvironment
from ..controller import ControllerBuilder

if TYPE_CHECKING:
    # Only imported for type checking, as the default sails depend on the lifting line module
    from ..default_sails import DefaultSail, SailDefinition, SolverType


class CompleteSailModelSettings(StormbirdSetupBaseModel):
    max_controller_iterations: int = 10
    allowed_angle_error: float = 0.00175 # 0.1 degrees in radians
    nr_loadings_to_test_during_optimization: int = 10
    retractable: bool = False
    thrust_direction: SpatialVector = Field(default_factory=lambda: SpatialVector(x=-1.0))

class CompleteSailModelBuilder(StormbirdSetupBaseModel):
    lifting_line_simulation: SimulationBuilder
    controller: ControllerBuilder
    wind_environment: WindEnvironment = Field(default_factory=lambda: WindEnvironment())
    settings: CompleteSailModelSettings = Field(default_factory=lambda: CompleteSailModelSettings())

    @classmethod
    def new_from_default_sail(
        cls,
        *,
        sail: "DefaultSail | SailDefinition",
        positions: list[SpatialVector],
        chord_length: float,
        height: float,
        wind_environment: WindEnvironment | None = None,
        symmetry_condition: SymmetryCondition = SymmetryCondition.NoSymmetry,
        settings: CompleteSailModelSettings | None = None,
        solver_type: "SolverType | None" = None
    ) -> "CompleteSailModelBuilder":
        '''
        Sets up a complete sail model with a sail at each of the given positions, where all sails
        use the same sail model and size. The sail can either be one of the default sails, or a
        custom sail definition. The solver type selects the version of a default sail, and is the
        linearized solver if not given. It can not be given for a custom sail definition, which
        already contains the solver settings.

        The positions are the bottom of each sail. The span of each sail goes along the up direction
        in the wind environment, and the chord along the zero direction vector, so that a local wing
        angle of zero aligns the sails with a head wind. If no settings are given, the thrust
        direction is opposite of the zero direction vector, which is the direction the ship sails in.

        A sail with its base on the symmetry plane continues as a mirror image below it, which is
        detected from the position. For instance, with the Z symmetry condition, a sail at z = 0
        stands on the symmetry plane, while a sail at z > 0 stands above it, e.g., on the deck.
        '''
        from ..default_sails import DefaultSail, SolverType

        if len(positions) == 0:
            raise ValueError("At least one sail position must be given")

        if isinstance(sail, DefaultSail):
            definition = sail.definition(
                SolverType.Linearized if solver_type is None else solver_type
            )
        elif solver_type is not None:
            raise ValueError(
                "The solver type can only be given for default sails, as a custom sail definition "
                "already contains the solver settings"
            )
        else:
            definition = sail

        if wind_environment is None:
            wind_environment = WindEnvironment()

        line_force_model = LineForceModelBuilder(
            circulation_correction = definition.circulation_correction.model_copy(deep=True)
        )

        for position in positions:
            line_force_model.add_wing_builder(
                definition.wing_builder(
                    position = position,
                    chord_length = chord_length,
                    height = height,
                    chord_direction = wind_environment.zero_direction_vector,
                    up_direction = wind_environment.up_direction,
                    base_on_symmetry_plane = _base_on_symmetry_plane(
                        position, wind_environment.up_direction, height, symmetry_condition
                    )
                )
            )

        lifting_line_simulation = SimulationBuilder(
            line_force_model = line_force_model,
            simulation_settings = QuasiSteadySettings(
                solver = definition.solver.model_copy(deep=True),
                wake = QuasiSteadyWakeSettings(symmetry_condition = symmetry_condition)
            )
        )

        controller = ControllerBuilder(
            set_points = [definition.scaled_controller_set_points(chord_length) for _ in positions]
        )

        if settings is None:
            settings = CompleteSailModelSettings(
                thrust_direction = wind_environment.zero_direction_vector.normalize() * -1.0
            )

        return cls(
            lifting_line_simulation = lifting_line_simulation,
            controller = controller,
            wind_environment = wind_environment,
            settings = settings
        )


def _base_on_symmetry_plane(
    position: SpatialVector,
    up_direction: SpatialVector,
    height: float,
    symmetry_condition: SymmetryCondition
) -> bool:
    '''
    Whether a sail with the bottom at the given position stands on the symmetry plane, which goes
    through the origin. This is the case when the position is on the plane, and the span of the sail
    is normal to the plane.
    '''
    match symmetry_condition:
        case SymmetryCondition.NoSymmetry:
            return False
        case SymmetryCondition.X:
            plane_normal = SpatialVector(x=1.0)
        case SymmetryCondition.Y:
            plane_normal = SpatialVector(y=1.0)
        case SymmetryCondition.Z:
            plane_normal = SpatialVector(z=1.0)

    position_on_plane = abs(position.dot(plane_normal)) < 1e-6 * height
    span_normal_to_plane = abs(abs(up_direction.normalize().dot(plane_normal)) - 1.0) < 1e-6

    return position_on_plane and span_normal_to_plane
