'''
Copyright (C) 2024, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
'''

from ..base_model import StormbirdSetupBaseModel

from enum import Enum

from pydantic import model_serializer

import numpy as np

class InputPowerData(StormbirdSetupBaseModel):
    section_models_internal_state_data: list[float]
    input_power_coefficient_data: list[float]

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

    @classmethod
    def new_polynomial_rotor_sail_model(
        cls,
        max_power: float,
        max_rps: float,
        area: float
    ) -> "InputPowerModel":
        '''
        Simple model for the power based on a polynomial relationship between the power and RPS.

        The polynomial power is set to 2.5, which comes from data fitted to data from the SWOPP
        project. However, the actual power is scaled based on the supplied values for max_power
        and max_rps.
        '''

        section_models_internal_state_data = np.linspace(0, max_rps, 20)

        power = 2.5

        factor = max_power / (max_rps**power * area)

        input_power_coefficient_data = factor * (section_models_internal_state_data**power)

        return cls(
            input_power_type = InputPowerDataType.FromInternalStateAlone,
            input_power_data = InputPowerData(
                section_models_internal_state_data = section_models_internal_state_data.tolist(),
                input_power_coefficient_data = input_power_coefficient_data.tolist()
            )
        )

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
