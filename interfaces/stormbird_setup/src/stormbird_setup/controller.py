"""
Copyright (C) 2024, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
"""

from .base_model import StormbirdSetupBaseModel

from pydantic import model_validator, Field

from enum import Enum

class InternalStateSetPointType(Enum):
    '''
    Defines the units of the internal state set point data:

    - SectionModel: the data is given in the units of the section model, e.g., spin ratio for rotor
      sails or the suction/power coefficient for suction sails, and is converted to the wing
      internal state.
    - Wing: the data is given directly as the wing internal state, e.g., the power (with the sign
      setting the direction) for suction sails, and is used without conversion.
    '''
    SectionModel = "SectionModel"
    Wing = "Wing"

class ControllerSetPoints(StormbirdSetupBaseModel):
    '''
    Set points for a sail, as a function of the apparent wind direction.

    The internal state data is given in the units specified by internal_state_set_point_type. The
    max value and max change rate for the internal state are applied to the wing internal state,
    after any conversion from the set point data.

    Note: the controller measures the apparent wind direction for each wing as the angle from the
    chord to the velocity, around the span direction of the wing. This is not the same as in the
    wind environment, which measures it around the wind rotation axis. For sails built from the
    bottom to the top, with the default wind environment, the two have opposite signs, so a wind
    from starboard is negative in the set points. Set points that are mirrored between positive and
    negative directions, such as for the default sails, are not affected. A fix is targeted for the
    next release of Stormbird.
    '''
    apparent_wind_directions_data: list[float]
    angle_of_attack_data: list[float] | None = None
    internal_state_data: list[float] | None = None
    internal_state_set_point_type: InternalStateSetPointType = InternalStateSetPointType.SectionModel
    wing_internal_state_max_value: float | None = None
    use_effective_angle_of_attack: bool = False
    max_local_wing_angle_change_rate: float | None = None
    wing_internal_state_max_change_rate: float | None = None


class SpanwiseMeasurementBuilder(StormbirdSetupBaseModel):
    '''
    Defines the part of the span of each wing that the controller measures the flow on. The two
    locations are non-dimensional, and run from -0.5 at one end of a wing to 0.5 at the other.

    Stormbird uses the control point that lies closest to each of the two locations, so a small
    deviation from them is expected: the control points are what a simulation resolves. Both of them
    are included in the measurement, so the same start and end location measures the single control
    point that lies closest to it. Each wing gets its own indices, since the wings can be built with
    a different number of sections.
    '''
    non_dim_start_location: float = -0.25
    non_dim_end_location: float = 0.25

    @model_validator(mode='after')
    def check_locations(self) -> "SpanwiseMeasurementBuilder":
        if self.non_dim_end_location < self.non_dim_start_location:
            raise ValueError(
                "The end of a spanwise measurement cannot be before its start. The start location "
                f"is {self.non_dim_start_location} and the end location is "
                f"{self.non_dim_end_location}"
            )

        return self

class ControllerBuilder(StormbirdSetupBaseModel):
    set_points: list[ControllerSetPoints]
    spanwise_measurement: SpanwiseMeasurementBuilder = Field(
        default_factory=lambda: SpanwiseMeasurementBuilder()
    )
    time_steps_between_updates: int = 1
    start_time: float = 0.0
    moving_average_window_size: int | None = None
    use_input_velocity_for_apparent_wind_direction: bool = False
