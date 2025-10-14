'''
Copyright (C) 2024, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
'''

from ..base_model import StormbirdSetupBaseModel

from enum import Enum

from pydantic import field_serializer, Field

class InternalStateType(Enum):
    Generic = "Generic"
    SpinRatio = "SpinRatio"

class SpinRatioConversion(StormbirdSetupBaseModel):
    diameter: float
    max_rps: float

class ControllerLogic(StormbirdSetupBaseModel):
    apparent_wind_directions_data: list[float]
    angle_of_attack_set_points_data: list[float] | None = None
    section_model_internal_state_set_points_data: list[float] | None = None
    internal_state_type: InternalStateType = InternalStateType.Generic
    internal_state_conversion: SpinRatioConversion | None = Field(default=None, exclude=True)
    use_effective_angle_of_attack: bool = False

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
    logic: ControllerLogic
    flow_measurement_settings: FlowMeasurementSettings = FlowMeasurementSettings()
    time_steps_between_updates: int = 1
    start_time: float = 0.0
    max_local_wing_angle_change_rate: float | None = None
    max_internal_section_state_change_rate: float | None = None
    moving_average_window_size: int | None = None
    use_input_velocity_for_apparent_wind_direction: bool = False


