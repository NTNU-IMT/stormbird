"""
Copyright (C) 2024, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
"""

from typing import Any
from enum import Enum

from ..base_model import StormbirdSetupBaseModel

from pydantic import model_serializer, model_validator

class SelfInducedVelocityCorrectionType(Enum):
    NoCorrection = "NoCorrection"
    MaxInducedVelocityMagnitudeRatio = "MaxInducedVelocityMagnitudeRatio"

class TotalVelocityCorrectionType(Enum):
    NoCorrection = "NoCorrection"
    MaxInducedVelocityMagnitudeRatio = "MaxInducedVelocityMagnitudeRatio"
    FixedMagnitudeEqualToFreestream = "FixedMagnitudeEqualToFreestream"

class _VelocityCorrectionBase(StormbirdSetupBaseModel):
    '''
    Shared serialization for the velocity corrections, which are enums in Stormbird. Variants
    without a value are serialized as a string, and variants with a value as a dictionary with the
    variant name as the key.
    '''
    @model_validator(mode='before')
    @classmethod
    def deserialize_velocity_correction(cls, data: Any) -> Any:
        if isinstance(data, str):
            return {"type": data, "value": None}

        if isinstance(data, dict) and 'type' not in data and len(data) == 1:
            ((key, value),) = data.items()

            return {"type": key, "value": value}

        return data

    @model_serializer
    def ser_model(self) -> dict[str, Any] | str:
        if self.value is None:
            return self.type.value

        return {self.type.value: self.value}

class SelfInducedVelocityCorrection(_VelocityCorrectionBase):
    '''
    Empirical correction of the self-induced velocities, i.e., the velocities induced on a wing by
    its own wake.
    '''
    type: SelfInducedVelocityCorrectionType = SelfInducedVelocityCorrectionType.NoCorrection
    value: float | None = None

    @classmethod
    def new_max_induced_velocity_magnitude(cls, max_magnitude_ratio: float = 1.0) -> "SelfInducedVelocityCorrection":
        return cls(
            type = SelfInducedVelocityCorrectionType.MaxInducedVelocityMagnitudeRatio,
            value = max_magnitude_ratio
        )

class TotalVelocityCorrection(_VelocityCorrectionBase):
    '''
    Empirical correction of the total velocity, i.e., after the self-induced and the other-induced
    velocities are added together.
    '''
    type: TotalVelocityCorrectionType = TotalVelocityCorrectionType.NoCorrection
    value: float | None = None

    @classmethod
    def new_max_induced_velocity_magnitude(cls, max_magnitude_ratio: float = 1.0) -> "TotalVelocityCorrection":
        return cls(
            type = TotalVelocityCorrectionType.MaxInducedVelocityMagnitudeRatio,
            value = max_magnitude_ratio
        )

    @classmethod
    def new_fixed_magnitude_equal_to_freestream(cls) -> "TotalVelocityCorrection":
        return cls(
            type = TotalVelocityCorrectionType.FixedMagnitudeEqualToFreestream,
            value = None
        )

class InducedVelocitySettings(StormbirdSetupBaseModel):
    '''
    How the induced velocities are combined with the freestream velocity in the lifting line
    solvers. The self-induced velocity is first neglected or corrected, then added to the
    other-induced velocity, before the total correction is applied.
    '''
    neglect_self_induced_velocities: bool = False
    self_induced_correction: SelfInducedVelocityCorrection = SelfInducedVelocityCorrection()
    total_correction: TotalVelocityCorrection = TotalVelocityCorrection()
