'''
Copyright (C) 2024, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
'''

from ..base_model import StormbirdSetupBaseModel

class InputPower(StormbirdSetupBaseModel):
    section_models_internal_state_data: list[float]
    input_power_per_wing_data: list[float]