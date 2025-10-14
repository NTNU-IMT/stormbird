'''
Copyright (C) 2024, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
'''


from pydantic import model_serializer, Field

from .base_model import StormbirdSetupBaseModel

from .spatial_vector import SpatialVector

class PowerHeightVariation(StormbirdSetupBaseModel):
    reference_height: float = 10.0
    power_factor: float = 1.0/9.0

class LogarithmicHeightVariation(StormbirdSetupBaseModel):
    reference_height: float = 10.0
    surface_roughness: float = 0.0002

class HeightVariationModel(StormbirdSetupBaseModel):
    model: PowerHeightVariation | LogarithmicHeightVariation = PowerHeightVariation()

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


class WindEnvironment(StormbirdSetupBaseModel):
    height_variation_model: HeightVariationModel | None = HeightVariationModel()
    up_direction: SpatialVector = Field(
        default_factory=lambda: SpatialVector(x=0.0, y=0.0, z=1.0)
    )
    wind_rotation_axis: SpatialVector = Field(
        default_factory=lambda: SpatialVector(x=0.0, y=0.0, z=-1.0)
    )
    zero_direction_vector: SpatialVector = Field(
        default_factory=lambda: SpatialVector(x=1.0, y=0.0, z=0.0)
    )
    water_plane_height: float = 0.0
    
    


