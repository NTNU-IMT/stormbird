import json
import numpy as np

from pystormbird.lifting_line import Simulation
from pystormbird import Vec3

from dataclasses import dataclass
from enum import Enum

class SimulationMode(Enum):
    DYNAMIC = 0
    STATIC = 1

@dataclass(frozen=True, kw_only=True)
class SimulationCase():
    angle_of_attack: float
    start_angle_of_attack: float | None = None
    chord_length: float = 11
    span: float = 33
    freestream_velocity: float = 8.0
    density: float = 1.225
    nr_sections: int = 32
    simulation_mode: SimulationMode = SimulationMode.STATIC
    smoothing_length: float | None = None
    circulation_viscosity: float | None = None
    section_model: dict | None = None
    z_symmetry: bool = False
    write_wake_files: bool = False

    @property
    def force_factor(self) -> float:
        return 0.5 * self.chord_length * self.span * self.density * self.freestream_velocity**2

def run_simulation(simulation_case: SimulationCase):
    freestream_velocity = Vec3(simulation_case.freestream_velocity, 0.0, 0.0)

    chord_vector = Vec3(simulation_case.chord_length, 0.0, 0.0)

    if simulation_case.section_model is None:
        section_model = {
            "Foil": {}
        }
    else:
        section_model = simulation_case.section_model

    wing_builder = {
        "section_points": [
            {"x": 0.0, "y": 0.0, "z": 0.0},
            {"x": 0.0, "y": 0.0, "z": simulation_case.span}
        ],
        "chord_vectors": [
            {"x": chord_vector.x, "y": chord_vector.y, "z": chord_vector.z},
            {"x": chord_vector.x, "y": chord_vector.y, "z": chord_vector.z}
        ],
        "section_model": section_model
    }

    line_force_model = {
        "wing_builders": [wing_builder],
        "nr_sections": simulation_case.nr_sections,
        "density": simulation_case.density,
    }

    gaussian_smoothing_settings = None
    artificial_viscosity = None
    if simulation_case.smoothing_length is not None:
        end_corrections = [(False, True)] if simulation_case.z_symmetry else [(True, True)]

        gaussian_smoothing_settings = {
            "length_factor": simulation_case.smoothing_length,
            "end_corrections": end_corrections
        }
    
    if simulation_case.circulation_viscosity is not None:
        artificial_viscosity = {
            "viscosity": simulation_case.circulation_viscosity,
            "solver_iterations": 20,
            "solver_damping": 0.1
        }
        

    if gaussian_smoothing_settings is not None or artificial_viscosity is not None:
        smoothing_settings = {}

        if gaussian_smoothing_settings is not None:
            smoothing_settings["gaussian"] = gaussian_smoothing_settings
        
        if artificial_viscosity is not None:
            smoothing_settings["artificial_viscosity"] = artificial_viscosity
    
        line_force_model["smoothing_settings"] = smoothing_settings

    solver = {
        "damping_factor_start": 0.01,
        "damping_factor_end": 0.1,
        "max_iterations_per_time_step": 3
    }

    end_time = 50 * simulation_case.chord_length / simulation_case.freestream_velocity
    dt = end_time / 500

    wake = {}

    if simulation_case.z_symmetry:
        wake["symmetry_condition"] = "Z"

    match simulation_case.simulation_mode:
        case SimulationMode.DYNAMIC:
            wake["ratio_of_wake_affected_by_induced_velocities"] = 0.75
            wake["first_panel_relative_length"] = 0.75
            wake["last_panel_relative_length"] = 50.0
            wake["use_chord_direction"] = True

            wake["wake_length"] = {
                "NrPanels": 60
            }

            wake["viscous_core_length_off_body"] = {
                "Absolute": 0.25 * simulation_case.chord_length
            }

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
        "write_wake_data_to_file": simulation_case.write_wake_files,
        "wake_files_folder_path": "wake_files_output"
    }

    setup_string = json.dumps(setup)

    angle_speed = 10 / (0.25 * end_time)

    simulation = Simulation(
        setup_string = setup_string,
        initial_time_step = dt,
        wake_initial_velocity = freestream_velocity
    )

    freestream_velocity_points = simulation.get_freestream_velocity_points()

    freestream_velocity_list = []
    for _ in freestream_velocity_points:
        freestream_velocity_list.append(
            freestream_velocity
        )

    current_time = 0.0

    result_history = []

    current_angle_deg = (
        simulation_case.angle_of_attack if simulation_case.start_angle_of_attack is None 
        else simulation_case.start_angle_of_attack
    )

    while current_time < end_time:
        simulation.set_local_wing_angles([-np.radians(current_angle_deg)])

        result = simulation.do_step(
            time = current_time, 
            time_step = dt, 
            freestream_velocity = freestream_velocity_list
        )

        current_time += dt

        result_history.append(result)

        if current_angle_deg < simulation_case.angle_of_attack:
            current_angle_deg += angle_speed * dt

        if current_angle_deg > simulation_case.angle_of_attack:
            current_angle_deg = simulation_case.angle_of_attack

    return result_history

    