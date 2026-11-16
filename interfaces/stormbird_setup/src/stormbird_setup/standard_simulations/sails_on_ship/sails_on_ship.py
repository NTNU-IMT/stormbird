"""
Copyright (C) 2026, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
"""

import math
from pathlib import Path
from typing import Any, Callable

from pydantic import Field, field_validator

from ...base_model import StormbirdSetupBaseModel
from ...spatial_vector import SpatialVector
from ...wind import PowerModel, WindCondition, WindEnvironment
from ...line_force_model import WingBuilder
from ...actuator_line import ActuatorLineBuilder
from ...stormflow.geometry import GeometryBuilder
from ... import openfoam
from ... import stormflow
from ...openfoam.results import ActuatorLineResults
from ..actuator_line_runs import run_openfoam_simulation, run_stormflow_simulation

from .coordinate_system import ForwardDirection, Origin
from .conditions import ShipTestCondition, WindProfile
from .naming import case_name, check_setup_name
from .simulation_settings import (
    CfdSolver, OpenFoamShipSettings, StormflowShipSettings, StormflowRunSettings
)


class SailsOnShip(StormbirdSetupBaseModel):
    '''
    Standard setup of actuator line simulations of sails on a ship.

    The ship coordinate system is given by the forward direction and the origin: the x axis points
    either forward or backward, the z axis points up, with z = 0 at the water plane, and the y axis
    completes a right-handed coordinate system. The wind environment and the linear velocity of the
    ship follow from this, so that a true wind direction of zero is a head wind.

    The actuator line setup contains the sails at their positions on the ship, and all settings for
    the actuator line model, including an optional controller. It is used as a template, which is
    copied and adjusted for each test condition.

    The simulation builders for OpenFOAM and Stormflow use standard domains and resolutions, given
    in ship lengths and cells per mean chord length, which can be changed through the settings for
    each solver.
    '''
    name: str
    """
    Name of the setup, e.g., the sails and the ship, such as "example_ship". Used
    as the first part of the case names, i.e., the names of the result files and the OpenFOAM case
    folders, so that the cases of different setups get different names. Can only contain letters,
    digits, "_", "-" and ".".
    """
    ship_length: float
    """Length between perpendiculars. Used to scale the size of the domain."""
    actuator_line: ActuatorLineBuilder
    forward_direction: ForwardDirection = ForwardDirection.PositiveX
    origin: Origin = Origin.AftPerpendicular
    ship_geometry_path: str | None = None
    """Optional surface mesh of the ship, e.g., an .obj file, in the ship coordinate system"""
    openfoam_settings: OpenFoamShipSettings = Field(default_factory=OpenFoamShipSettings)
    stormflow_settings: StormflowShipSettings = Field(default_factory=StormflowShipSettings)
    openfoam_run_folder: str | None = None
    """
    Folder where the OpenFOAM cases are written and run, each in a sub folder with the case name.
    None means the current working directory.
    """

    @field_validator("name")
    @classmethod
    def check_name(cls, name: str) -> str:
        return check_setup_name(name)

    def case_name(self, condition: ShipTestCondition, solver: CfdSolver) -> str:
        '''
        The standard name of a case, starting with the name of the setup. See naming.py for the
        naming convention.
        '''
        return case_name(solver, condition, self.name)

    def result_file_name(self, condition: ShipTestCondition, solver: CfdSolver) -> str:
        '''The name of the JSON file with the results of a case'''
        return self.case_name(condition, solver) + ".json"

    def openfoam_case_path(self, condition: ShipTestCondition) -> Path:
        '''The folder where the OpenFOAM case is written and run'''
        run_folder = Path.cwd() if self.openfoam_run_folder is None else Path(self.openfoam_run_folder)

        return run_folder / self.case_name(condition, CfdSolver.OpenFOAM)

    def wind_environment(self) -> WindEnvironment:
        '''
        Wind environment with z up, and the zero direction vector pointing backward, i.e., the
        direction a head wind blows in. The wind direction is measured clockwise seen from above,
        so that positive directions are wind from starboard, for both forward directions.
        '''
        return WindEnvironment(
            zero_direction_vector = self.forward_direction.vector() * -1.0,
            up_direction = SpatialVector(z=1.0)
        )

    def ship_x_range(self) -> tuple[float, float]:
        '''The smallest and largest x coordinate of the ship, from the aft to the forward perpendicular'''
        return self.origin.ship_x_range(self.ship_length, self.forward_direction)

    @staticmethod
    def wind_condition(condition: ShipTestCondition) -> WindCondition:
        '''
        The true wind, with the wind profile in the condition. With the power law profile, the true
        wind velocity in the condition is the velocity at 10 m above the water plane.
        '''
        velocity_variation: PowerModel | float

        match condition.wind_profile:
            case WindProfile.PowerLaw:
                velocity_variation = PowerModel(reference_velocity = condition.true_wind_velocity)
            case WindProfile.Constant:
                velocity_variation = float(condition.true_wind_velocity)
            case _:
                raise ValueError("Unsupported wind profile:", condition.wind_profile)

        return WindCondition(
            direction_coming_from = condition.true_wind_direction,
            velocity_variation = velocity_variation
        )

    def linear_velocity(self, condition: ShipTestCondition) -> SpatialVector:
        '''
        The linear velocity used for the apparent wind, which is the true wind plus this velocity.
        It therefore points backward, opposite of the motion of the ship, as in
        WindEnvironment.linear_velocity_from_ship_velocity.
        '''
        return self.wind_environment().linear_velocity_from_ship_velocity(
            float(condition.ship_velocity)
        )

    def apparent_wind_at_mid_span(self, condition: ShipTestCondition) -> list[SpatialVector]:
        '''The undisturbed apparent wind velocity at the mid-span of each wing'''
        wind_environment = self.wind_environment()
        wind_condition = self.wind_condition(condition)
        linear_velocity = self.linear_velocity(condition)

        return [
            wind_environment.steady_apparent_wind_velocity_vector_at_location(
                wind_condition, _mid_span_point(wing_builder), linear_velocity
            )
            for wing_builder in self.actuator_line.line_force_model.wing_builders
        ]

    def apparent_wind_velocity_at_sails(self, condition: ShipTestCondition) -> float:
        '''The magnitude of the undisturbed apparent wind at mid-span, averaged over all wings'''
        velocities = [velocity.length() for velocity in self.apparent_wind_at_mid_span(condition)]

        return sum(velocities) / len(velocities)

    def stormflow_run_settings(self, condition: ShipTestCondition) -> StormflowRunSettings:
        '''
        Run settings for a Stormflow simulation of the test condition. The end time is the time the
        apparent wind at the sails uses to travel the number of ship lengths in the Stormflow
        settings.
        '''
        settings = self.stormflow_settings

        apparent_wind_velocity = self.apparent_wind_velocity_at_sails(condition)

        if apparent_wind_velocity <= 0.0:
            raise ValueError(
                "The apparent wind velocity at the sails is zero, so the end time can not be set"
            )

        end_time = settings.ship_lengths_of_travel * self.ship_length / apparent_wind_velocity

        return StormflowRunSettings(
            end_time = end_time,
            courant_number = settings.courant_number,
            full_result_interval = end_time / settings.nr_full_results,
        )

    def thrust_signs(self, condition: ShipTestCondition) -> list[float]:
        '''
        The sign of the lift coefficient that gives thrust for each wing, which is the sign that
        angles of attack and internal states are multiplied with.

        Positive lift points along the span direction crossed with the apparent wind velocity, so
        the lift gives thrust when this vector has a positive component along the forward direction.
        A pure head or tail wind gives no thrust from lift, and the sign is then positive.
        '''
        forward = self.forward_direction.vector()

        return [
            thrust_sign(forward, wing_builder, apparent_wind)
            for wing_builder, apparent_wind in zip(
                self.actuator_line.line_force_model.wing_builders,
                self.apparent_wind_at_mid_span(condition)
            )
        ]

    def local_wing_angles(self, condition: ShipTestCondition, angle_of_attack: float) -> list[float]:
        '''
        Local wing angles that give the angle of attack, relative to the undisturbed apparent wind
        at the mid-span of each wing, with the sign set so that a positive angle gives thrust.

        The angle of attack is the right-handed angle around the span from the chord to the apparent
        wind, so the local wing angle is the angle of the apparent wind minus the angle of attack.
        The local wing angle is zero when the chord is as given in the actuator line setup. The
        results are kept between -pi and pi.
        '''
        angles = []

        for wing_builder, apparent_wind, sign in zip(
            self.actuator_line.line_force_model.wing_builders,
            self.apparent_wind_at_mid_span(condition),
            self.thrust_signs(condition)
        ):
            span_direction = _span_direction(wing_builder)

            chord = _perpendicular_part(wing_builder.chord_vectors[0], span_direction)
            apparent_wind_in_plane = _perpendicular_part(apparent_wind, span_direction)

            wind_angle = chord.signed_angle_between(apparent_wind_in_plane, span_direction)

            angle = wind_angle - sign * angle_of_attack

            angles.append(math.atan2(math.sin(angle), math.cos(angle)))

        return angles

    def actuator_line_for_condition(self, condition: ShipTestCondition) -> ActuatorLineBuilder:
        '''
        A copy of the actuator line setup, adjusted to the test condition:

        - With a fixed angle of attack, the local wing angles are set to give that angle, and the
          controller is removed, so that the angles are kept constant.
        - With a fixed internal state, the internal states are set, with the sign that gives thrust.

        Without any of these, the copy is the same as the actuator line setup.
        '''
        actuator_line = self.actuator_line.model_copy(deep=True)

        line_force_model = actuator_line.line_force_model

        if condition.angle_of_attack is not None:
            line_force_model.local_wing_angles = self.local_wing_angles(
                condition, condition.angle_of_attack
            )
            actuator_line.controller = None

        if condition.internal_state is not None:
            line_force_model.internal_states = [
                sign * condition.internal_state for sign in self.thrust_signs(condition)
            ]

        return actuator_line

    def geometries(self) -> list[GeometryBuilder]:
        '''The ship geometry as a no-slip geometry, if given'''
        if self.ship_geometry_path is None:
            return []

        return [GeometryBuilder.new_triangle_mesh(str(self.ship_geometry_path))]

    def simulation_builder(
        self,
        condition: ShipTestCondition,
        solver: CfdSolver,
        actuator_line: ActuatorLineBuilder | None = None
    ) -> openfoam.SimulationBuilder | stormflow.SimulationBuilder:
        '''
        The simulation builder for the test condition with the given solver. The actuator line
        setup is from actuator_line_for_condition, unless another setup is given, e.g., with a
        controller that is adjusted to the condition.
        '''
        match solver:
            case CfdSolver.OpenFOAM:
                return self.openfoam_simulation_builder(condition, actuator_line)
            case CfdSolver.Stormflow:
                return self.stormflow_simulation_builder(condition, actuator_line)
            case _:
                raise ValueError("Unsupported CFD solver:", solver)

    def openfoam_simulation_builder(
        self,
        condition: ShipTestCondition,
        actuator_line: ActuatorLineBuilder | None = None
    ) -> openfoam.SimulationBuilder:
        '''OpenFOAM simulation builder for the test condition. See simulation_builder.'''
        settings = self.openfoam_settings

        start_point, end_point = settings.domain.bounds(
            self.ship_length, self.ship_x_range(), self.forward_direction
        )

        return openfoam.SimulationBuilder(
            grid = openfoam.GridBuilder(
                start_point = start_point,
                end_point = end_point,
                background_mesh_cells_per_representative_length = settings.background_cells_per_chord,
                actuator_line_refinement = openfoam.RefinementBuilder(
                    cells_per_representative_length = settings.actuator_line_cells_per_chord,
                    padding = settings.actuator_line_refinement_padding
                ),
                geometry_cells_per_representative_length = settings.ship_cells_per_chord,
            ),
            wind_environment = self.wind_environment(),
            wind_condition = self.wind_condition(condition),
            linear_velocity = self.linear_velocity(condition),
            actuator_line = self._actuator_line(condition, actuator_line),
            geometries = self.geometries(),
            time_control = settings.time_control.model_copy(deep=True),
        )

    def stormflow_simulation_builder(
        self,
        condition: ShipTestCondition,
        actuator_line: ActuatorLineBuilder | None = None
    ) -> stormflow.SimulationBuilder:
        '''
        Stormflow simulation builder for the test condition. See simulation_builder. The velocity
        and pressure solvers run on the compute platform in the settings.
        '''
        settings = self.stormflow_settings

        start_point, end_point = settings.domain.bounds(
            self.ship_length, self.ship_x_range(), self.forward_direction
        )

        compute_platform = settings.compute_platform

        return stormflow.SimulationBuilder(
            grid = stormflow.GridBuilder(
                start_point = start_point,
                end_point = end_point,
                cells_per_representative_length = (settings.cells_per_chord,) * 3,
            ),
            wind_environment = self.wind_environment(),
            wind_condition = self.wind_condition(condition),
            linear_velocity = self.linear_velocity(condition),
            actuator_line = self._actuator_line(condition, actuator_line),
            geometries = self.geometries(),
            effective_viscosity = settings.effective_viscosity,
            velocity_solver = stormflow.VelocitySolverBuilder(compute_platform = compute_platform),
            pressure_solver = stormflow.PressureSolverBuilder(
                settings = stormflow.MultigridSettingsBuilder(compute_platform = compute_platform)
            ),
            turbulence = None if settings.turbulence is None else settings.turbulence.model_copy(deep=True),
        )

    def run(
        self,
        condition: ShipTestCondition,
        solver: CfdSolver,
        *,
        actuator_line: ActuatorLineBuilder | None = None,
        openfoam_case_path: Path | str | None = None,
        openfoam_nr_processes: int | None = None,
        progress_postfix: Callable[[Any], str] | None = None,
    ) -> ActuatorLineResults:
        '''
        Runs the test condition with the given solver, and returns the results in the same format
        for both solvers. Requires the solver to be available: OpenFOAM loaded in the terminal, or
        pystormflow installed.

        - OpenFOAM: the case is written to openfoam_case_path, which is deleted first if it exists.
          The default is the folder from the openfoam_case_path method, i.e., a folder with the
          case name in the OpenFOAM run folder.
        - Stormflow: the end time, Courant number and interval between the full results are from
          stormflow_run_settings. progress_postfix is shown after the progress bar, see
          run_stormflow_simulation.

        The actuator line setup is from actuator_line_for_condition, unless another setup is given.
        '''
        simulation_builder = self.simulation_builder(condition, solver, actuator_line)

        match solver:
            case CfdSolver.OpenFOAM:
                assert isinstance(simulation_builder, openfoam.SimulationBuilder)

                if openfoam_case_path is None:
                    openfoam_case_path = self.openfoam_case_path(condition)

                return run_openfoam_simulation(
                    simulation_builder, openfoam_case_path, openfoam_nr_processes
                )
            case CfdSolver.Stormflow:
                assert isinstance(simulation_builder, stormflow.SimulationBuilder)

                run_settings = self.stormflow_run_settings(condition)

                return run_stormflow_simulation(
                    simulation_builder,
                    end_time = run_settings.end_time,
                    courant_number = run_settings.courant_number,
                    full_result_interval = run_settings.full_result_interval,
                    progress_postfix = progress_postfix,
                )
            case _:
                raise ValueError("Unsupported CFD solver:", solver)

    def _actuator_line(
        self,
        condition: ShipTestCondition,
        actuator_line: ActuatorLineBuilder | None
    ) -> ActuatorLineBuilder:
        if actuator_line is None:
            return self.actuator_line_for_condition(condition)

        return actuator_line


def thrust_sign(
    forward: SpatialVector,
    wing_builder: WingBuilder,
    apparent_wind: SpatialVector
) -> float:
    '''
    The sign of the lift coefficient that gives thrust for a wing in the given apparent wind. See
    SailsOnShip.thrust_signs.
    '''
    thrust_component = forward.dot(_span_direction(wing_builder).cross(apparent_wind))

    return -1.0 if thrust_component < 0.0 else 1.0


def _non_virtual_end_points(wing_builder: WingBuilder) -> tuple[SpatialVector, SpatialVector]:
    '''
    The first and last section points of the part of the wing that is not virtual, i.e., the wing
    itself without virtual extensions such as end plates.
    '''
    points = wing_builder.section_points
    is_virtual = wing_builder.line_segment_is_virtual or [False] * (len(points) - 1)

    real_segments = [index for index, virtual in enumerate(is_virtual) if not virtual]

    if len(real_segments) == 0:
        raise ValueError("A wing must have at least one line segment that is not virtual")

    return points[real_segments[0]], points[real_segments[-1] + 1]


def _mid_span_point(wing_builder: WingBuilder) -> SpatialVector:
    start, end = _non_virtual_end_points(wing_builder)

    return (start + end) * 0.5


def _span_direction(wing_builder: WingBuilder) -> SpatialVector:
    start, end = _non_virtual_end_points(wing_builder)

    return (end - start).normalize()


def _perpendicular_part(vector: SpatialVector, unit_axis: SpatialVector) -> SpatialVector:
    '''The part of the vector that is perpendicular to the unit axis'''
    return vector - unit_axis * vector.dot(unit_axis)
