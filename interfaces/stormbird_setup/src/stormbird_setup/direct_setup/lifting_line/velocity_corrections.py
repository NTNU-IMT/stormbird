
'''
Copyright (C) 2024, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
'''

from typing import Any
from enum import Enum

from ...base_model import StormbirdSetupBaseModel

from pydantic import model_serializer, model_validator

class VelocityCorrectionType(Enum):
    NoCorrection = "NoCorrection"
    MaxInducedVelocityMagnitudeRatio = "MaxInducedVelocityMagnitudeRatio"
    FixedMagnitudeEqualToFreestream = "FixedMagnitudeEqualToFreestream"

class VelocityCorrections(StormbirdSetupBaseModel):
    type: VelocityCorrectionType = VelocityCorrectionType.NoCorrection
    value: float | None = None
    
    @classmethod
    def new_max_induced_velocity_magnitude(cls, max_magnitude_ratio: float = 1.0):
        return cls(
            type = VelocityCorrectionType.MaxInducedVelocityMagnitudeRatio,
            value = max_magnitude_ratio
        )
        
    @classmethod
    def new_fixed_magnitude_equal_to_freestream(cls):
        return cls(
            type = VelocityCorrectionType.FixedMagnitudeEqualToFreestream,
            value = None
        )
        
    @model_validator(mode='before')
    @classmethod
    def deserialize_velocity_correction(cls, data: Any) -> Any:
        # If already in correct format, return as-is
        if isinstance(data, dict) and 'type' in data:
            return data
        
        # Handle string formats
        if isinstance(data, str):
            if data == "NoCorrection":
                return {"type": VelocityCorrectionType.NoCorrection, "value": None}
            elif data == "FixedMagnitudeEqualToFreestream":
                return {"type": VelocityCorrectionType.FixedMagnitudeEqualToFreestream, "value": None}
        
        # Handle dict format with MaxInducedVelocityMagnitudeRatio
        if isinstance(data, dict) and "MaxInducedVelocityMagnitudeRatio" in data:
            return {
                "type": VelocityCorrectionType.MaxInducedVelocityMagnitudeRatio,
                "value": data["MaxInducedVelocityMagnitudeRatio"]
            }
        
        return data

    @model_serializer
    def ser_model(self):
        if self.type == VelocityCorrectionType.NoCorrection:
            return "NoCorrection"
        elif self.type == VelocityCorrectionType.MaxInducedVelocityMagnitudeRatio:
            return {
                "MaxInducedVelocityMagnitudeRatio": self.value
            }
        elif self.type == VelocityCorrectionType.FixedMagnitudeEqualToFreestream:
            return "FixedMagnitudeEqualToFreestream"
        else:
            raise ValueError(f"Unknown velocity correction type: {self.type}")