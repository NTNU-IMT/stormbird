'''
Copyright (C) 2024, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
'''

from .base_model import StormbirdSetupBaseModel

from pydantic import model_serializer

class Foil(StormbirdSetupBaseModel):
    cl_zero_angle: float | None = None
    cl_initial_slope: float | None = None
    cl_high_order_factor: float | None = None
    cl_high_order_power: float | None = None
    cl_max_after_stall: float | None = None
    cd_min: float | None = None
    angle_cd_min: float | None = None
    cd_second_order_factor: float | None = None 
    cd_max_after_stall: float | None = None
    cd_power_after_stall: float | None = None
    cdi_correction_factor: float | None = None
    mean_positive_stall_angle: float | None = None
    mean_negative_stall_angle: float | None = None
    stall_range: float | None = None
    added_mass_factor: float | None = None

class VaryingFoil(StormbirdSetupBaseModel):
    internal_state_data: list[float]
    foils_data: list[Foil]
    current_internal_state: float | None = None

class RotatingCylinder(StormbirdSetupBaseModel):
    revolutions_per_second: float | None = None
    spin_ratio_data: list[float] | None = None
    cl_data: list[float]| None = None
    cd_data: list[float] | None = None
    added_mass_factor: float | None = None

class SectionModel(StormbirdSetupBaseModel):
    model: Foil | VaryingFoil | RotatingCylinder = Foil()

    @model_serializer
    def ser_model(self):
        model_dict = self.model.model_dump(exclude_none=True)

        if isinstance(self.model, Foil):
            return {
                "Foil": model_dict
            }
        elif isinstance(self.model, VaryingFoil):
            return {
                "VaryingFoil": model_dict
            }
        elif isinstance(self.model, RotatingCylinder):
            return {
                "RotatingCylinder": model_dict
            }
        else:
            raise ValueError("Unsupported section model:", type(self.model))