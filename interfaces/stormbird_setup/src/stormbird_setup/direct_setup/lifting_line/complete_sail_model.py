
'''
Copyright (C) 2024, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
'''

from ...base_model import StormbirdSetupBaseModel

from .simulation_builder import SimulationBuilder
from ..wind import WindEnvironment
from ..controller import ControllerBuilder
from ..input_power import InputPower

class CompleteSailModelBuilder(StormbirdSetupBaseModel):
    lifting_line_simulation: SimulationBuilder
    controller: ControllerBuilder
    wind_environment: WindEnvironment = WindEnvironment()
    input_power: InputPower | None = None