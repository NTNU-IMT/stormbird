
from pydantic import model_serializer

from ..base_model import StormbirdSetupBaseModel
from ..line_force_model import LineForceModelBuilder

from .solver import Linearized, SimpleIterative
from .wake import QuasiSteadyWake, DynamicWake

class QuasiSteadySettings(StormbirdSetupBaseModel):
    solver: Linearized | SimpleIterative = Linearized()
    wake: QuasiSteadyWake = QuasiSteadyWake()

    @model_serializer
    def ser_model(self):
        solver_dict = self.solver.model_dump()
        wake_dict = self.wake.model_dump()

        if isinstance(self.solver, Linearized):
            return {
                "solver": {
                    "Linearized": solver_dict
                },
                "wake": wake_dict
            }
        elif isinstance(self.solver, SimpleIterative):
            return {
                "solver": {
                    "SimpleIterative": solver_dict
                },
                "wake": wake_dict
            }

class DynamicSettings(StormbirdSetupBaseModel):
    solver: Linearized | SimpleIterative = Linearized()
    wake: DynamicWake = DynamicWake()

    @model_serializer
    def ser_model(self):
        solver_dict = self.solver.model_dump()
        wake_dict = self.wake.model_dump()

        if isinstance(self.solver, Linearized):
            return {
                "solver": {
                    "Linearized": solver_dict
                },
                "wake": wake_dict
            }
        elif isinstance(self.solver, SimpleIterative):
            return {
                "solver": {
                    "SimpleIterative": solver_dict
                },
                "wake": wake_dict
            }

class SimulationBuilder(StormbirdSetupBaseModel):
    line_force_model: LineForceModelBuilder = LineForceModelBuilder()
    simulation_settings: QuasiSteadySettings | DynamicSettings = QuasiSteadySettings()

    @model_serializer
    def ser_model(self):
        line_force_model_dict = self.line_force_model.model_dump()
        simulation_settings_dict = self.simulation_settings.model_dump()

        if isinstance(self.simulation_settings, QuasiSteadySettings):
            return {
                "line_force_model": line_force_model_dict,
                "simulation_settings": {
                    "QuasiSteady": simulation_settings_dict
                }
            }
        elif isinstance(self.simulation_settings, DynamicSettings):
            return {
                "line_force_model": line_force_model_dict,
                "simulation_settings": {
                    "Dynamic": simulation_settings_dict
                }
            }
        


