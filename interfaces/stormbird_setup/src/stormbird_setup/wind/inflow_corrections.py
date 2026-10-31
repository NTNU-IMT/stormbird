"""
Copyright (C) 2024, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
"""

from ..base_model import StormbirdSetupBaseModel
from ..spatial_vector import SpatialVector
from ..range import Range

from typing import Any, TYPE_CHECKING

if TYPE_CHECKING:
    # Only imported for type checking, as the wind environment depends on this module
    from .wind_environment import WindEnvironment
    from .wind_condition import WindCondition

class InflowCorrectionsSingleDirection(StormbirdSetupBaseModel):
    height_values: list[float]
    magnitude_corrections: list[float]
    angle_corrections: list[float]
    wing_indices: list[Range]

    @classmethod
    def from_simulation_result(
        cls,
        stormbird_result: dict[str, Any],
        wind_environment: "WindEnvironment",
        wind_condition: "WindCondition",
        ship_velocity: float = 0.0
    ) -> "InflowCorrectionsSingleDirection":
        '''
        Computes inflow corrections from velocity measurements at the control points, by comparing
        the measured velocity with the undisturbed apparent wind at the same locations. The typical
        use is a CFD simulation of a ship without sails, where an actuator line with the
        EffectiveWindSensor section model samples the velocity at the sail positions.

        The undisturbed apparent wind is computed from the wind environment, the wind condition and
        the ship velocity, so it should match the inflow conditions in the CFD simulation, including
        any variation with height. The ship velocity is along the zero direction vector, as in the
        complete sail model, see `WindEnvironment.linear_velocity_from_ship_velocity`.

        Only the part of the measured velocity that is normal to the up direction is used. The
        magnitude correction is the ratio between the magnitude of this part and the magnitude of
        the undisturbed apparent wind, and the angle correction is the angle from the undisturbed
        apparent wind to the measured velocity, around the up direction. This is the same way as the
        corrections are applied in Stormbird. The wings do not need to be aligned with the flow.

        The corrections are given at the height of each control point. The apparent wind direction
        that the corrections belong to can be computed with
        `WindEnvironment.apparent_wind_direction_from_condition_and_linear_velocity`.

        Args:
            stormbird_result: a simulation result from Stormbird, as a dictionary, e.g., read from
                one of the full result files from an actuator line simulation. The control points and
                the sampled velocity in the result are always in the global coordinate system.
            wind_environment: the wind environment
            wind_condition: the wind condition in the simulation
            ship_velocity: the ship velocity in the simulation
        '''
        ctrl_points = [
            SpatialVector.from_dict(point) for point in stormbird_result["ctrl_points"]
        ]

        measured_velocity = [
            SpatialVector.from_dict(velocity)
            for velocity in stormbird_result["solver_input_ctrl_points_velocity"]
        ]

        wing_indices = [
            Range(start = indices["start"], end = indices["end"])
            for indices in stormbird_result["wing_indices"]
        ]

        up_direction = wind_environment.up_direction.normalize()

        linear_velocity = wind_environment.linear_velocity_from_ship_velocity(ship_velocity)

        height_values = []
        magnitude_corrections = []
        angle_corrections = []

        for ctrl_point, velocity in zip(ctrl_points, measured_velocity):
            # The same height as used when the corrections are applied in Stormbird
            height_values.append(ctrl_point.dot(up_direction))

            undisturbed_velocity = wind_environment.steady_apparent_wind_velocity_vector_at_location(
                wind_condition,
                ctrl_point,
                linear_velocity
            )

            undisturbed_velocity = undisturbed_velocity - up_direction * undisturbed_velocity.dot(up_direction)

            horizontal_velocity = velocity - up_direction * velocity.dot(up_direction)

            if undisturbed_velocity.length() == 0.0:
                raise ValueError(
                    "The undisturbed apparent wind is zero at a control point, so the inflow "
                    "corrections can not be computed"
                )

            magnitude_corrections.append(
                horizontal_velocity.length() / undisturbed_velocity.length()
            )

            angle_corrections.append(
                undisturbed_velocity.signed_angle_between(horizontal_velocity, up_direction)
            )

        return cls(
            height_values = height_values,
            magnitude_corrections = magnitude_corrections,
            angle_corrections = angle_corrections,
            wing_indices = wing_indices
        )

class InflowCorrections(StormbirdSetupBaseModel):
    apparent_wind_directions: list[float]
    corrections: list[InflowCorrectionsSingleDirection]
