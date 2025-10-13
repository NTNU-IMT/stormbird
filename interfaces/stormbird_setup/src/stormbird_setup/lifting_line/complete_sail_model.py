
from ..base_model import StormbirdSetupBaseModel

from .simulation_builder import SimulationBuilder
from ..wind import WindEnvironment
from ..controller import ControllerBuilder
from ..input_power import InputPower

class CompleteSailModelBuilder(StormbirdSetupBaseModel):
    lifting_line_simulation: SimulationBuilder
    controller: ControllerBuilder
    wind_environment: WindEnvironment = WindEnvironment()
    input_power: InputPower | None = None