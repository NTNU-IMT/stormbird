"""
Copyright (C) 2024, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
"""

import math
from pydantic import model_serializer, model_validator

from enum import Enum

from ...base_model import StormbirdSetupBaseModel

class PowerHeightVariation(StormbirdSetupBaseModel):
    reference_height: float = 10.0
    power_factor: float = 1.0/9.0
    
class AtmosphereState(Enum):
    Neutral = "Neutral"
    Stable = "Stable"
    Unstable = "Unstable"

class LogarithmicHeightVariation(StormbirdSetupBaseModel):
    friction_velocity: float
    surface_roughness: float
    von_karman_constant: float = 0.41
    obukhov_length: float | None = None
    atmosphere_state: AtmosphereState = AtmosphereState.Stable

class HeightVariationModel(StormbirdSetupBaseModel):
    model: PowerHeightVariation | LogarithmicHeightVariation = PowerHeightVariation()
    
    @model_validator(mode='before')
    @classmethod
    def deserialize_from_rust_enum(cls, data):
        if not isinstance(data, dict):
            return data
            
        if not data:
            return data
        
        # Already in Python/Pydantic form
        if "model" in data:
            return data
        
        # Rust externally-tagged enum format: {"VariantName": {fields...}}
        if "PowerModel" in data:
            return {"model": PowerHeightVariation(**data["PowerModel"])}
        elif "LogarithmicModel" in data:
            return {"model": LogarithmicHeightVariation(**data["LogarithmicModel"])}
        else:
            raise ValueError(f"Unknown height variation model variant: {list(data.keys())}")
    
    
    @classmethod
    def new_logarithmic(cls, *, friction_velocity: float, surface_roughness: float, von_karman_constant: float = 0.41):
        return cls(
            model = LogarithmicHeightVariation(
                friction_velocity = friction_velocity, 
                surface_roughness = surface_roughness,
                von_karman_constant = von_karman_constant
            )
        )
        
    @classmethod
    def new_logarithmic_from_reference_and_friction_velocity(
        cls, 
        *, 
        reference_velocity: float, 
        friction_velocity: float,
        reference_height: float = 10.0
    ):
        von_karman_constant = 0.41

        exp_factor = reference_velocity * von_karman_constant / friction_velocity
        
        surface_roughness = reference_height / math.exp(exp_factor)
        
        return cls(
            model = LogarithmicHeightVariation(
                friction_velocity = friction_velocity, 
                surface_roughness = surface_roughness
            )
        )
        
    @classmethod
    def new_power(cls, *, reference_height: float = 10.0, power_factor: float = 1.0/9.0):
        return cls(
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
