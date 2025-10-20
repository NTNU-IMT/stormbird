'''
Copyright (C) 2024, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
'''

from ...base_model import StormbirdSetupBaseModel

from enum import Enum

from pydantic import model_serializer

class SymmetryCondition(Enum):
    NoSymmetry = "NoSymmetry"
    Z = "Z"
    Y = "Y"
    X = "X"

class ViscousCoreLengthType(Enum):
    Relative = "Relative"
    Absolute = "Absolute"
    NoViscousCore = "NoViscousCore"

class ViscousCoreLength(StormbirdSetupBaseModel):
    value_type: ViscousCoreLengthType = ViscousCoreLengthType.Relative
    value: float = 0.1

    @model_serializer
    def ser_model(self):
        match self.value_type:
            case ViscousCoreLengthType.Relative:
                return {
                    "Relative": self.value
                }
            case ViscousCoreLengthType.Absolute:
                return {
                    "Absolute": self.value
                }
            case ViscousCoreLengthType.NoViscousCore:
                return "NoViscousCore"
            case _:
                raise ValueError("Invalid ViscousCoreLengthType")

class QuasiSteadyWakeSettings(StormbirdSetupBaseModel):
    wake_length_factor: float = 100.0
    symmetry_condition: SymmetryCondition = SymmetryCondition.NoSymmetry
    viscous_core_length: ViscousCoreLength = ViscousCoreLength()


class DynamicWakeBuilder(StormbirdSetupBaseModel):
    nr_panels_per_line_element: int = 100
    viscous_core_length: ViscousCoreLength = ViscousCoreLength()
    symmetry_condition: SymmetryCondition = SymmetryCondition.NoSymmetry
    first_panel_relative_length: float = 0.75
    last_panel_relative_length: float = 25.0
    use_chord_direction: bool = False
    write_wake_data_to_file: bool = False
    wake_files_folder_path: str = ""
