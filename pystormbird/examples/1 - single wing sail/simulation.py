import json
import numpy as np

from pystormbird.lifting_line import Simulation
from pystormbird import SpatialVector

from dataclasses import dataclass
from enum import Enum

class SimulationMode(Enum):
    DYNAMIC = 0
    STATIC = 1

@dataclass(frozen=True, kw_only=True)
class SimulationCase():
    '''
    This class is responsible for setting up and running a simulation case.

    As input, it requires choices about which "mode" to run the simulation in, as well as the 
    parameters of the wing.
    '''
    angle_of_attack: float
    section_model_dict: dict
    chord_length: float = 1.0
    span: float = 4.5
    freestream_velocity: float = 8.0
    density: float = 1.225
    nr_sections: int = 64
    simulation_mode: SimulationMode = SimulationMode.STATIC
    smoothing_length: float | None = None
    z_symmetry: bool = False
    write_wake_files: bool = False
    prescribed_circulation: bool = False
    prescribed_initialization: bool = False

    @property
    def force_factor(self) -> float:
        return 0.5 * self.chord_length * self.span * self.density * self.freestream_velocity**2
    
    def run(self):
        freestream_velocity = SpatialVector(self.freestream_velocity, 0.0, 0.0)

        chord_vector = SpatialVector(self.chord_length, 0.0, 0.0)

        non_zero_circulation_at_ends = [True, False] if self.z_symmetry else [False, False]

        wing_builder = {
            "section_points": [
                {"x": 0.0, "y": 0.0, "z": 0.0},
                {"x": 0.0, "y": 0.0, "z": self.span}
            ],
            "chord_vectors": [
                {"x": chord_vector.x, "y": chord_vector.y, "z": chord_vector.z},
                {"x": chord_vector.x, "y": chord_vector.y, "z": chord_vector.z}
            ],
            "section_model": self.section_model_dict,
            "non_zero_circulation_at_ends": non_zero_circulation_at_ends
        }

        line_force_model = {
            "wing_builders": [wing_builder],
            "nr_sections": self.nr_sections,
            "density": self.density,
        }

        if self.smoothing_length is not None and not(self.prescribed_circulation):
            gaussian_smoothing = {
                "length_factor": self.smoothing_length,
            }

            line_force_model["circulation_corrections"] = {
                "GaussianSmoothing": gaussian_smoothing
            }

        if self.prescribed_circulation:
            line_force_model["circulation_corrections"] = {
                "PrescribedCirculation": {
                    "outer_power": 0.2
                }
            }
            

        solver = {
            "max_iterations_per_time_step": 10,
            "damping_factor": 0.1
        } if self.simulation_mode == SimulationMode.DYNAMIC else {
            "max_iterations_per_time_step": 1000,
            "damping_factor": 0.05
        }   

        end_time = 10 * self.chord_length / self.freestream_velocity
        dt = end_time / 1000

        wake = {}

        if self.z_symmetry:
            wake["symmetry_condition"] = "Z"

        match self.simulation_mode:
            case SimulationMode.DYNAMIC:
                sim_settings = {
                    "Dynamic": {
                        "solver": solver,
                        "wake": wake,
                    }
                }
            case SimulationMode.STATIC:
                sim_settings = {
                    "QuasiSteady": {
                        "solver": solver,
                        "wake": wake
                    }
                }
            case _:
                raise ValueError("Invalid simulation type")

        setup = {
            "line_force_model": line_force_model,
            "simulation_mode": sim_settings,
            "write_wake_data_to_file": self.write_wake_files,
            "wake_files_folder_path": "wake_files_output"
        }

        setup_string = json.dumps(setup)

        simulation = Simulation(
            setup_string = setup_string,
            initial_time_step = dt,
            initialization_velocity = freestream_velocity
        )

        freestream_velocity_points = simulation.get_freestream_velocity_points()

        freestream_velocity_list = []
        for _ in freestream_velocity_points:
            freestream_velocity_list.append(
                freestream_velocity
            )

        current_time = 0.0

        result_history = []

        simulation.set_local_wing_angles([-np.radians(self.angle_of_attack)])

        if self.simulation_mode == SimulationMode.DYNAMIC:
            while current_time < end_time:
                result = simulation.do_step(
                    time = current_time, 
                    time_step = dt, 
                    freestream_velocity = freestream_velocity_list
                )

                current_time += dt

                result_history.append(result)
        else:
            simulation.set_local_wing_angles([-np.radians(self.angle_of_attack)])

            if self.prescribed_initialization:
                simulation.initialize_with_elliptic_distribution(
                    time = current_time,
                    time_step = end_time,
                    freestream_velocity = freestream_velocity_list
                )

            result = simulation.do_step(
                time = current_time, 
                time_step = end_time, 
                freestream_velocity = freestream_velocity_list
            )

            result_history.append(result)

        return result_history

    