"""
Copyright (C) 2024, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
"""


from pydantic import model_serializer, Field

from ...base_model import StormbirdSetupBaseModel

from ..spatial_vector import SpatialVector

class PowerHeightVariation(StormbirdSetupBaseModel):
    reference_height: float = 10.0
    power_factor: float = 1.0/9.0

class LogarithmicHeightVariation(StormbirdSetupBaseModel):
    reference_height: float = 10.0
    surface_roughness: float = 0.0002

class HeightVariationModel(StormbirdSetupBaseModel):
    model: PowerHeightVariation | LogarithmicHeightVariation = PowerHeightVariation()
    
    @classmethod
    def new_logarithmic(cls, reference_height: float = 10.0, surface_roughness: float = 0.0002):
        return cls(
            model = LogarithmicHeightVariation(
                reference_height=reference_height, 
                surface_roughness=surface_roughness
            )
        )
        
    @classmethod
    def new_power(cls, reference_height: float = 10.0, power_factor: float = 1.0/9.0):
        cls(
            model = PowerHeightVariation(
                reference_height = reference_height,
                power_factor = power_factor
            )
        )

    @model_serializer
    def ser_model(self):
        model_dict = self.model.model_dump()

        if isinstance(self.model, PowerHeightVariation):
            return {
                "PowerModel": model_dict
            }
        elif isinstance(self.model, LogarithmicHeightVariation):
            return {
                "LogarithmicModel": model_dict
            }
        else:
            raise ValueError("Unsupported height variation model:", type(self.model))