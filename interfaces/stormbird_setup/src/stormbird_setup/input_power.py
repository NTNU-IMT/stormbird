"""
Copyright (C) 2024, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
"""

from .base_model import StormbirdSetupBaseModel

from enum import Enum
from typing import Any

from pydantic import model_serializer, model_validator

import numpy as np

class InputPowerData(StormbirdSetupBaseModel):
    internal_state_data: list[float]
    input_power_data: list[float]

class InputPowerDataType(Enum):
    FromSectionModelPowerCoefficient = "FromSectionModelPowerCoefficient"
    NoPower = "NoPower"
    DirectFromInternalState = "DirectFromInternalState"
    InterpolateFromInternalState = "InterpolateFromInternalState"

class InputPowerModel(StormbirdSetupBaseModel):
    '''
    Interface to the input power model. The variants are:

    - FromSectionModelPowerCoefficient (default): the power is calculated from the power
      coefficient of the section models, integrated over the wing. Section models that do not need
      power, such as regular foils, have a power coefficient of zero.
    - NoPower: the wing does not need power.
    - DirectFromInternalState: the absolute value of the wing internal state is the power.
    - InterpolateFromInternalState: the power is interpolated from data, as a function of the
      absolute value of the wing internal state.
    '''
    input_power_type: InputPowerDataType = InputPowerDataType.FromSectionModelPowerCoefficient
    input_power_data: InputPowerData | None = None

    @classmethod
    def new_polynomial_rotor_sail_model(
        cls,
        max_power: float,
        max_rps: float,
        poly_power: float = 2.5
    ) -> "InputPowerModel":
        '''
        Simple model for the power based on a polynomial relationship between the power and RPS,
        which is the internal state of a rotor sail.

        The polynomial power is set to 2.5, which comes from data fitted to data from the SWOPP
        project. However, the actual power is scaled based on the supplied values for max_power
        and max_rps.
        '''

        internal_state_data = np.linspace(0, max_rps, 20)

        input_power_data = max_power * (internal_state_data / max_rps)**poly_power

        return cls(
            input_power_type = InputPowerDataType.InterpolateFromInternalState,
            input_power_data = InputPowerData(
                internal_state_data = internal_state_data.tolist(),
                input_power_data = input_power_data.tolist()
            )
        )

    @model_validator(mode='before')
    @classmethod
    def deserialize_from_rust_enum(cls, data: Any) -> Any:
        # Unit variants are represented as strings
        if isinstance(data, str):
            return {
                'input_power_type': InputPowerDataType(data),
                'input_power_data': None
            }

        if not isinstance(data, dict):
            return data

        # Empty dict means use defaults
        if not data:
            return data

        # Already in Python/Pydantic form
        if 'input_power_type' in data or 'input_power_data' in data:
            return data

        # Rust externally-tagged enum format
        if 'InterpolateFromInternalState' in data:
            return {
                'input_power_type': InputPowerDataType.InterpolateFromInternalState,
                'input_power_data': InputPowerData(**data['InterpolateFromInternalState'])
            }
        else:
            raise ValueError(f"Unknown input power model variant: {list(data.keys())}")

    @model_serializer
    def ser_model(self) -> dict[str, object] | str:
        match self.input_power_type:
            case InputPowerDataType.FromSectionModelPowerCoefficient | \
                InputPowerDataType.NoPower | \
                InputPowerDataType.DirectFromInternalState:
                return self.input_power_type.value
            case InputPowerDataType.InterpolateFromInternalState:
                if self.input_power_data is None:
                    raise ValueError("input_power_data must be set for InterpolateFromInternalState")

                return {
                    "InterpolateFromInternalState": self.input_power_data.model_dump()
                }
            case _:
                raise ValueError("Unknown input power type", self.input_power_type)
