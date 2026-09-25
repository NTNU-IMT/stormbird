"""
Copyright (C) 2024, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
"""

from .base_model import StormbirdSetupBaseModel

from enum import Enum

from pydantic import field_serializer, model_validator, Field

import numpy as np

class InternalStateType(Enum):
    Generic = "Generic"
    SpinRatio = "SpinRatio"

class SpinRatioConversion(StormbirdSetupBaseModel):
    diameter: float
    max_rps: float

class ControllerSetPoints(StormbirdSetupBaseModel):
    apparent_wind_directions_data: list[float]
    angle_of_attack_data: list[float] | None = None
    section_model_internal_state_data: list[float] | None = None
    internal_state_type: InternalStateType = InternalStateType.Generic
    internal_state_conversion: SpinRatioConversion | None = Field(default=None, exclude=True)
    use_effective_angle_of_attack: bool = False
    max_local_wing_angle_change_rate: float | None = None
    max_internal_section_state_change_rate: float | None = None

    @field_serializer('internal_state_type')
    def serialize_internal_state_type(self, value: InternalStateType):
        match value:
            case InternalStateType.Generic:
                return "Generic"
            case InternalStateType.SpinRatio:
                if self.internal_state_conversion is None:
                    raise ValueError("SpinRatioConversion must be provided for SpinRatio internal state type.")
                return {
                    "SpinRatio": self.internal_state_conversion.model_dump()
                }
            case _:
                raise ValueError("Unsupported internal state type:", value)
                
    @classmethod
    def new_default_wing_sail_single_element(cls, max_angle_deg: float = 15.0):
        apparent_wind_directions_data = np.radians([-180, -15, -10, 10, 15, 180])
        angle_of_attack_data = np.radians([-max_angle_deg, -max_angle_deg, 0.0, 0.0, max_angle_deg, max_angle_deg])

        return ControllerSetPoints(
            apparent_wind_directions_data = apparent_wind_directions_data.tolist(),
            angle_of_attack_data = angle_of_attack_data.tolist()
        )
        
    @classmethod
    def new_default_wing_sail_two_element(cls, max_angle_deg: float = 12.0):
        apparent_wind_directions_data = np.radians([-180, -15, -10, 10, 15, 180])
        angle_of_attack_data = np.radians([-max_angle_deg, -max_angle_deg, 0.0, 0.0, max_angle_deg, max_angle_deg])
        section_model_internal_state_data = np.radians([-30.0, -30.0, 0.0, 0.0, 30.0, 30.0])

        return ControllerSetPoints(
            apparent_wind_directions_data = apparent_wind_directions_data.tolist(),
            angle_of_attack_data = angle_of_attack_data.tolist(),
            section_model_internal_state_data = section_model_internal_state_data.tolist()
        )
        
    @classmethod
    def new_default_rotor_sail(cls, *, diameter: float, max_rps: float):
        '''
        Helper function to quickly set up a suitable controller for a rotor sail. Assumed to be
        fairly general
        '''
        apparent_wind_directions_data = np.radians([-180, -40, -15, 15, 40, 180])
        section_model_internal_state_data = [3.0, 3.0, 0.0, 0.0, -3.0, -3.0]

        internal_state_type = InternalStateType.SpinRatio
        internal_state_conversion = SpinRatioConversion(
            diameter = diameter,
            max_rps = max_rps
        )

        return ControllerSetPoints(
            apparent_wind_directions_data = apparent_wind_directions_data.tolist(),
            section_model_internal_state_data = section_model_internal_state_data,
            internal_state_type = internal_state_type,
            internal_state_conversion = internal_state_conversion
        )
        
    @classmethod
    def new_default_suction_sail(cls, max_aoa_deg: float=30.0, max_ca: float = 0.3):        
        apparent_wind_directions_data = np.radians([-180, -15, -10, 10, 15, 180]).tolist()
        
        angle_of_attack_data = np.radians([
            -max_aoa_deg, -max_aoa_deg, 0.0, 
            0.0, max_aoa_deg, max_aoa_deg
        ]).tolist()
        
        section_model_internal_state_data = [
            -max_ca, -max_ca, 0.0, 
            0.0, max_ca, max_ca
        ]

        return ControllerSetPoints(
            apparent_wind_directions_data = apparent_wind_directions_data,
            angle_of_attack_data = angle_of_attack_data,
            section_model_internal_state_data = section_model_internal_state_data
        )


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
