"""
Copyright (C) 2024, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
"""

from pydantic import Field

from ..base_model import StormbirdSetupBaseModel
from ..spatial_vector import SpatialVector
from .simulation_builder import SimulationBuilder
from ..wind import WindEnvironment
from ..controller import ControllerBuilder


class CompleteSailModelSettings(StormbirdSetupBaseModel):
    max_controller_iterations: int = 10
    allowed_angle_error: float = 0.00175 # 0.1 degrees in radians
    nr_loadings_to_test_during_optimization: int = 10
    retractable: bool = False
    thrust_direction: SpatialVector = Field(default_factory=lambda: SpatialVector(x=-1.0))

class CompleteSailModelBuilder(StormbirdSetupBaseModel):
    lifting_line_simulation: SimulationBuilder
    controller: ControllerBuilder
    wind_environment: WindEnvironment = Field(default_factory=lambda: WindEnvironment())
    settings: CompleteSailModelSettings = Field(default_factory=lambda: CompleteSailModelSettings())
