
'''
Copyright (C) 2024, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
'''

from ...base_model import StormbirdSetupBaseModel

from enum import Enum

from pydantic import model_serializer

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