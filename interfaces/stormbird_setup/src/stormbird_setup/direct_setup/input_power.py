'''
Copyright (C) 2024, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
'''

from ..base_model import StormbirdSetupBaseModel

from enum import Enum

from pydantic import model_serializer

class InputPowerData(StormbirdSetupBaseModel):
    section_models_internal_state_data: list[float]
    input_power_per_wing_data: list[float]

class InputPowerDataType(Enum):
    NoPower = "NoPower"
    FromInternalStateAlone = "FromInternalStateAlone"
    FromInternalStateAndVelocity = "FromInternalStateAndVelocity"

class InputPowerModel(StormbirdSetupBaseModel):
    '''
    Interface to the input power model
    '''
    input_power_type: InputPowerDataType = InputPowerDataType.NoPower
    input_power_data: InputPowerData | None = None

    @model_serializer
    def ser_model(self) -> dict[str, object] | str:
        match self.input_power_type:
            case InputPowerDataType.NoPower:
                return self.input_power_type.value
            case InputPowerDataType.FromInternalStateAlone:
                if self.input_power_data is None:
                    raise ValueError("input_power_data must be set for FromInternalStateAlone")

                return {
                    "FromInternalStateAlone": self.input_power_data.model_dump()
                }
            case InputPowerDataType.FromInternalStateAndVelocity:
                if self.input_power_data is None:
                    raise ValueError("input_power_data must be set for FromInternalStateAndVelocity")

                return {
                    "FromInternalStateAndVelocity": self.input_power_data.model_dump()
                }
