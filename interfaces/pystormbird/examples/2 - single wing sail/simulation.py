import json
import numpy as np

from pystormbird.lifting_line import Simulation
from pystormbird import SpatialVector as SpatialVectorRust

from stormbird_setup.direct_setup.spatial_vector import SpatialVector
from stormbird_setup.direct_setup.line_force_model import LineForceModelBuilder, WingBuilder
from stormbird_setup.direct_setup.lifting_line.simulation_builder import SimulationBuilder, QuasiSteadySettings, DynamicSettings
from stormbird_setup.direct_setup.lifting_line.solver import Linearized, SimpleIterative
from stormbird_setup.direct_setup.lifting_line.wake import QuasiSteadyWake, DynamicWake, SymmetryCondition
from stormbird_setup.direct_setup.section_models import SectionModel

from enum import Enum

class SolverType(Enum):
    Linearized = "Linearized"
    SimpleIterative = "SimpleIterative"

from dataclasses import dataclass

@dataclass(frozen=True, kw_only=True)
class SimulationCase():
    '''
    This class is responsible for setting up and running a simulation case.

    As input, it requires choices about which "mode" to run the simulation in, as well as the 
    parameters of the wing.
    '''
    angle_of_attack: float
    section_model: SectionModel
    chord_length: float = 1.0
    span: float = 4.5
    freestream_velocity: float = 8.0
    density: float = 1.225
    nr_sections: int = 32
    dynamic: bool = False
    solver_type: SolverType = SolverType.Linearized
    z_symmetry: bool = False

    @property
    def force_factor(self) -> float:
        return 0.5 * self.chord_length * self.span * self.density * self.freestream_velocity**2
    
    def get_line_force_model(self) -> LineForceModelBuilder:
        chord_vector = SpatialVector(x=self.chord_length, y=0.0, z=0.0)

        non_zero_circulation_at_ends = (True, False) if self.z_symmetry else (False, False)

        wing_builder = WingBuilder(
            section_points = [
                SpatialVector(x=0.0, y=0.0, z=0.0),
                SpatialVector(x=0.0, y=0.0, z=self.span)
            ],
            chord_vectors = [
                chord_vector,
                chord_vector
            ],
            section_model = self.section_model,
            non_zero_circulation_at_ends = non_zero_circulation_at_ends
        )

        line_force_model = LineForceModelBuilder(
            wing_builders = [wing_builder],
            nr_sections = self.nr_sections,
            density = self.density
        )

        return line_force_model
    
    @property
    def end_time(self) -> float:
        return 40 * self.chord_length / self.freestream_velocity
    
    @property
    def time_step(self) -> float:
        return 0.25 * self.chord_length / self.freestream_velocity
    
    def simulation_builder_str(self) -> str:
        line_force_model = self.get_line_force_model()
        
        symmetry_condition = SymmetryCondition.Z if self.z_symmetry else SymmetryCondition.NoSymmetry
    
        if self.dynamic:
            match self.solver_type:
                case SolverType.SimpleIterative:
                    solver = SimpleIterative(
                        max_iterations_per_time_step = 20,
                        damping_factor = 0.1,
                    )
                case SolverType.Linearized:
                    solver = Linearized()

            wake = DynamicWake(
                symmetry_condition=symmetry_condition,
            )
            
            simulation_builder = SimulationBuilder(
                line_force_model = line_force_model,
                simulation_settings = DynamicSettings(
                    solver = solver,
                    wake = wake
                )
            )
        else:
            match self.solver_type:
                case SolverType.SimpleIterative:
                    solver = SimpleIterative(
                        max_iterations_per_time_step = 1000,
                        damping_factor = 0.05,
                    )
                case SolverType.Linearized:
                    solver = Linearized()

            wake = QuasiSteadyWake(
                symmetry_condition=symmetry_condition,
            )

            simulation_builder = SimulationBuilder(
                line_force_model = line_force_model,
                simulation_settings = QuasiSteadySettings(
                    solver = solver,
                    wake = wake
                )
            )

        return simulation_builder.to_json_string()
    
    def run(self):
        freestream_velocity = SpatialVectorRust(self.freestream_velocity, 0.0, 0.0)

        setup_string = self.simulation_builder_str()

        simulation = Simulation(setup_string = setup_string)

        freestream_velocity_points = simulation.get_freestream_velocity_points()

        freestream_velocity_list = []
        for _ in freestream_velocity_points:
            freestream_velocity_list.append(
                freestream_velocity
            )

        current_time = 0.0

        result_history = []

        simulation.set_local_wing_angles([-np.radians(self.angle_of_attack)])

        if self.dynamic:
            while current_time < self.end_time:
                result = simulation.do_step(
                    time = current_time, 
                    time_step = self.time_step, 
                    freestream_velocity = freestream_velocity_list
                )

                current_time += self.time_step

                result_history.append(result)
        else:
            simulation.set_local_wing_angles([-np.radians(self.angle_of_attack)])

            result = simulation.do_step(
                time = current_time, 
                time_step = self.time_step, 
                freestream_velocity = freestream_velocity_list
            )

            result_history.append(result)

        return result_history

    