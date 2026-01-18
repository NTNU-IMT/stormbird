'''
Copyright (C) 2024, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
'''
from typing import Any

from pydantic import model_serializer, model_validator

from ...base_model import StormbirdSetupBaseModel
from ..line_force_model import LineForceModelBuilder

from .solver import Linearized, SimpleIterative
from .wake import QuasiSteadyWakeSettings, DynamicWakeBuilder

class QuasiSteadySettings(StormbirdSetupBaseModel):
    solver: Linearized | SimpleIterative = Linearized()
    wake: QuasiSteadyWakeSettings  = QuasiSteadyWakeSettings ()
    
    @model_validator(mode='before')
    @classmethod
    def deserialize_solver(cls, data: Any) -> Any:
        if isinstance(data, dict) and 'solver' in data:
            solver_data = data['solver']
            if isinstance(solver_data, dict):
                if 'Linearized' in solver_data:
                    data['solver'] = Linearized(**solver_data['Linearized'])
                elif 'SimpleIterative' in solver_data:
                    data['solver'] = SimpleIterative(**solver_data['SimpleIterative'])
        return data

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
    solver: SimpleIterative | Linearized = SimpleIterative()
    wake: DynamicWakeBuilder = DynamicWakeBuilder()
    
    @model_validator(mode='before')
    @classmethod
    def deserialize_solver(cls, data: Any) -> Any:
        if isinstance(data, dict) and 'solver' in data:
            solver_data = data['solver']
            if isinstance(solver_data, dict):
                if 'Linearized' in solver_data:
                    data['solver'] = Linearized(**solver_data['Linearized'])
                elif 'SimpleIterative' in solver_data:
                    data['solver'] = SimpleIterative(**solver_data['SimpleIterative'])
        return data

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
    
    @model_validator(mode='before')
    @classmethod
    def deserialize_simulation_settings(cls, data: Any) -> Any:
        if isinstance(data, dict) and 'simulation_settings' in data:
            settings_data = data['simulation_settings']
            if isinstance(settings_data, dict):
                if 'QuasiSteady' in settings_data:
                    data['simulation_settings'] = QuasiSteadySettings(**settings_data['QuasiSteady'])
                elif 'Dynamic' in settings_data:
                    data['simulation_settings'] = DynamicSettings(**settings_data['Dynamic'])
        return data

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
