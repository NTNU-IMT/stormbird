'''
Simulation setup to reproduce the lifting line results from the paper "Actuator line for wind 
propulsion modelling", found here: 
https://www.researchgate.net/publication/374976524_Actuator_Line_for_Wind_Propulsion_Modelling
'''

from dataclasses import dataclass
import json

import numpy as np

from pystormbird.lifting_line import Simulation
from pystormbird import SpatialVector

@dataclass
class SimulationCase:
    angle_of_attack_deg: float
    wind_angle_deg: float = 45.0
    wind_speed: float = 12.0
    chord_length: float = 6.0
    span: float = 24.0
    start_height: float = 8.1
    nr_sections: int = 40
    density = 1.225
    nr_dynamic_wake_panels_per_section: int = 2
    dynamic: bool = False
    write_wake_files: bool = False

    @property
    def force_factor(self) -> float:
        return 0.5 * self.chord_length * self.span * self.density * self.wind_speed**2

    def get_line_force_model(self) -> dict:
        chord_vector = SpatialVector(self.chord_length, 0.0, 0.0)

        non_zero_circulation_at_ends = [False, False]

        wing_builders = []

        x_coordinates = [0.0, 0.0]
        y_coordinates = [-6.0, 6.0]

        for x, y in zip(x_coordinates, y_coordinates):

            wing_builders.append(
                {
                    "section_points": [
                        {"x": x, "y": y, "z": self.start_height},
                        {"x": x, "y": y, "z": self.start_height + self.span}
                    ],
                    "chord_vectors": [
                        {"x": chord_vector.x, "y": chord_vector.y, "z": chord_vector.z},
                        {"x": chord_vector.x, "y": chord_vector.y, "z": chord_vector.z}
                    ],
                    "section_model": {
                        "Foil": {
                            "cd_zero_angle": 0.01,
                            "mean_positive_stall_angle": np.radians(45.0), # Set large value to 'turn off' stall
                            "mean_negative_stall_angle": np.radians(45.0), # Set large value to 'turn off' stall
                        }
                    },
                    "non_zero_circulation_at_ends": non_zero_circulation_at_ends
                }
            )

        line_force_model = {
            "wing_builders": wing_builders,
            "nr_sections": self.nr_sections,
            "density": self.density,
        }

        return line_force_model
    
    def angle_of_attack(self):
        return np.radians(self.wind_angle_deg - self.angle_of_attack_deg)
    
    def run(self):
        freestream_velocity = SpatialVector(self.wind_speed, 0.0, 0.0)

        line_force_model = self.get_line_force_model()

        

        if self.dynamic:
            solver = {
                "max_iterations_per_time_step": 10,
                "damping_factor": 0.1,
            }
                
            wake = {
                "wake_length": {
                    "NrPanels": self.nr_dynamic_wake_panels_per_section
                },
                "use_chord_direction": True,
                "symmetry_condition": "Z"
            }

            sim_settings = {
                "Dynamic": {
                    "solver": solver,
                    "wake": wake
                }
            }
        else:
            sim_settings = {
                "QuasiSteady": {}
            }

        setup = {
            "line_force_model": line_force_model,
            "simulation_mode": sim_settings,
            "write_wake_data_to_file": self.write_wake_files,
            "wake_files_folder_path": "wake_files_output"
        }

        dt = 0.1
        nr_time_steps = 100 if self.dynamic else 1

        simulation = Simulation(
            setup_string = json.dumps(setup),
            initial_time_step = dt,
            initialization_velocity = freestream_velocity
        )

        freestream_velocity_points = simulation.get_freestream_velocity_points()

        freestream_velocity_list = []
        for _ in freestream_velocity_points:
            freestream_velocity_list.append(
                freestream_velocity
            )

        nr_wings = len(line_force_model["wing_builders"])
        angles_of_attack = np.ones(nr_wings) * self.angle_of_attack()

        simulation.set_local_wing_angles(angles_of_attack.tolist())
        simulation.set_rotation(SpatialVector(0.0, 0.0, -np.radians(self.wind_angle_deg)))

        for i in range(nr_time_steps):
            result = simulation.do_step(
                time = i * dt,
                time_step = dt,
                freestream_velocity = freestream_velocity_list
            )

        return result


