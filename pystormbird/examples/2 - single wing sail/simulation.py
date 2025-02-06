import json
import numpy as np

from pystormbird.lifting_line import Simulation
from pystormbird import SpatialVector

from dataclasses import dataclass
from enum import Enum

class SimulationMode(Enum):
    '''
    Enum used to choose between dynamic and static simulations.
    '''
    DYNAMIC = 0
    STATIC = 1

    def to_string(self):
        return self.name.replace("_", " ").lower()

class TestCase(Enum):
    '''
    Enum used to predefine settings for different test cases.
    '''
    RAW_SIMULATION = 0           # No corrections applied to the estimated circulation distribution
    PRESCRIBED_CIRCULATION = 1   # Circulation is prescribed to a fixed mathematical shape
    INITIALIZED_SIMULATION = 2   # Simulation is initialized with a prescribed circulation distribution, but then simulated without any corrections
    SMOOTHED = 3                 # Simulation is smoothed using a Gaussian kernel

    def to_string(self):
        return self.name.replace("_", " ").lower()
    
    @property
    def prescribed_circulation(self) -> bool:
        '''
        Returns True if the test case is prescribed circulation, False otherwise.
        '''
        match self:
            case TestCase.PRESCRIBED_CIRCULATION:
                return True
            case _:
                return False
    
    @property
    def prescribed_initialization(self) -> bool:
        '''
        Returns True if the test case is prescribed initialization, False otherwise.
        '''
        match self:
            case TestCase.INITIALIZED_SIMULATION:
                return True
            case _:
                return False
            
    @property
    def smoothing_length(self) -> float | None:
        '''
        Returns the smoothing length for the test case, or None if no smoothing is applied.
        '''
        match self:
            case TestCase.SMOOTHED:
                return 0.1
            case _:
                return None

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
    nr_sections: int = 32
    simulation_mode: SimulationMode = SimulationMode.STATIC
    smoothing_length: float | None = None
    z_symmetry: bool = False
    write_wake_files: bool = False
    prescribed_circulation: bool = False
    prescribed_initialization: bool = False

    @property
    def force_factor(self) -> float:
        return 0.5 * self.chord_length * self.span * self.density * self.freestream_velocity**2
    
    def get_line_force_model(self) -> dict:
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
                "end_corrections_delta_span_factor": 1.0,
                "end_corrections_number_of_insertions": 3
            }

            line_force_model["circulation_corrections"] = {
                "GaussianSmoothing": gaussian_smoothing
            }

        if self.prescribed_circulation:
            line_force_model["circulation_corrections"] = {
                "PrescribedCirculation": {
                    "inner_power": 2.5,
                    "outer_power": 0.2 # Note: The default also works ok. This Factor is tuned manually, based on manual comparison with a 'raw' simulation below stall.
                }
            }

        return line_force_model
    
    @property
    def end_time(self) -> float:
        return 40 * self.chord_length / self.freestream_velocity
    
    @property
    def time_step(self) -> float:
        return 0.25 * self.chord_length / self.freestream_velocity

    
    def run(self):
        freestream_velocity = SpatialVector(self.freestream_velocity, 0.0, 0.0)

        line_force_model = self.get_line_force_model()
            
        solver = {
            "max_iterations_per_time_step": 10,
            "damping_factor": 0.1
        } if self.simulation_mode == SimulationMode.DYNAMIC else {
            "max_iterations_per_time_step": 1000,
            "damping_factor": 0.05
        }   

        wake = {}

        if self.z_symmetry:
            wake["symmetry_condition"] = "Z"

        match self.simulation_mode:
            case SimulationMode.DYNAMIC:
                #wake["viscous_core_length_separated"] = {
                #    "Absolute": 1.0 * self.chord_length
                #}

                wake["strength_damping"] = "DirectFromStall"

                wake["wake_length"] = {
                    "NrPanels": 200
                }
                wake["use_chord_direction"] = True
                wake["first_panel_relative_length"] = 0.75
                

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
            initial_time_step = self.time_step,
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

            if self.prescribed_initialization:
                simulation.initialize_with_elliptic_distribution(
                    time = current_time,
                    time_step = self.time_step,
                    freestream_velocity = freestream_velocity_list
                )

            result = simulation.do_step(
                time = current_time, 
                time_step = self.time_step, 
                freestream_velocity = freestream_velocity_list
            )

            result_history.append(result)

        return result_history

    