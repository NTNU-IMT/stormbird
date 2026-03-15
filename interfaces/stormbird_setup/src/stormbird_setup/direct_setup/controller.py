'''
Copyright (C) 2024, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
'''

from ..base_model import StormbirdSetupBaseModel

from enum import Enum

from pydantic import field_serializer, Field

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
    def new_default_wing_sail_single_element(cls):
        apparent_wind_directions_data = np.radians([-180, -20, -10, 20, 30, 180])
        angle_of_attack_data = np.radians([-15.0, -15.0, 0.0, 0.0, 15, 15])

        return ControllerSetPoints(
            apparent_wind_directions_data = apparent_wind_directions_data.tolist(),
            angle_of_attack_data = angle_of_attack_data.tolist()
        )
        
    @classmethod
    def new_default_wing_sail_two_element(cls):
        apparent_wind_directions_data = np.radians([-180, -20, -10, 10, 20, 180])
        angle_of_attack_data = np.radians([-12.0, -12.0, 0.0, 0.0, 12, 12])
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
    def new_default_suction_sail(cls):
        max_aoa_deg = 30
        max_ca = 0.3
        
        apparent_wind_directions_data = np.radians([-180, -20, -10, 10, 20, 180]).tolist()
        
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


class MeasurementType(Enum):
    Mean = "Mean"
    Max = "Max"
    Min = "Min"

class MeasurementSettings(StormbirdSetupBaseModel):
    measurement_type: MeasurementType = MeasurementType.Mean
    start_index: int = 1
    end_offset: int = 1

class FlowMeasurementSettings(StormbirdSetupBaseModel):
    angle_of_attack: MeasurementSettings = MeasurementSettings()
    wind_direction: MeasurementSettings = MeasurementSettings()
    wind_velocity: MeasurementSettings = MeasurementSettings()

class ControllerBuilder(StormbirdSetupBaseModel):
    set_points: list[ControllerSetPoints]
    flow_measurement_settings: FlowMeasurementSettings = FlowMeasurementSettings()
    time_steps_between_updates: int = 1
    start_time: float = 0.0
    moving_average_window_size: int | None = None
    use_input_velocity_for_apparent_wind_direction: bool = False
        
    
