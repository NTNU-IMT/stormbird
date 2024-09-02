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
class RotorSimulationCase():
    spin_ratio: float
    diameter: float = 5.0
    height: float = 30.0
    freestream_velocity: float = 8.0
    density: float = 1.225
    nr_sections: int = 25
    smoothing_length: float | None = None
    simulation_mode: SimulationMode = SimulationMode.STATIC
    smoothing_length: float | None = None
    circulation_viscosity: float | None = None
    z_symmetry: bool = True
    virtual_end_disks: (bool, bool) = (False, True)
    virtual_end_disk_height_factor: float = 0.5
    only_consider_change_in_angle: bool = False
    write_wake_files: bool = False
    spin_ratio_data: list | None = None
    cd_data: list | None = None
    cl_data: list | None = None

    @property
    def force_factor(self) -> float:
        return 0.5 * self.diameter * self.height * self.density * self.freestream_velocity**2
    
    @property
    def revolutions_per_second(self) -> float:
        circumference = np.pi * self.diameter
        tangential_velocity = self.freestream_velocity * self.spin_ratio
                
        revolutions_per_second = -tangential_velocity / circumference 

        return revolutions_per_second
    
    def get_line_force_model(self):
        chord_vector = SpatialVector(self.diameter, 0.0, 0.0)

        section_model = {
            "RotatingCylinder": {
                "revolutions_per_second": float(self.revolutions_per_second)
            }
        }

        if self.spin_ratio_data is not None and self.cl_data is not None and self.cd_data is not None:
            if len(self.spin_ratio_data) != len(self.cl_data):
                raise ValueError("Section data input does not have the same length")
            
            section_model["RotatingCylinder"]["spin_ratio_data"] = self.spin_ratio_data
            section_model["RotatingCylinder"]["cl_data"] = self.cl_data
            section_model["RotatingCylinder"]["cd_data"] = self.cd_data
        
        rotor_builder = {
            "section_points": [
                {"x": 0.0, "y": 0.0, "z": 0.0},
                {"x": 0.0, "y": 0.0, "z": self.height}
            ],
            "chord_vectors": [
                {"x": chord_vector.x, "y": chord_vector.y, "z": chord_vector.z},
                {"x": chord_vector.x, "y": chord_vector.y, "z": chord_vector.z}
            ],
            "section_model": section_model
        }

        wing_builders = [rotor_builder]

        if self.virtual_end_disks[0]:
            virtual_end_sections = {
                "section_points": [
                    {"x": 0.0, "y": 0.0, "z": -self.virtual_end_disk_height_factor * self.diameter},
                    {"x": 0.0, "y": 0.0, "z": 0.0}
                ],
                "chord_vectors": [
                    {"x": chord_vector.x, "y": chord_vector.y, "z": chord_vector.z},
                    {"x": chord_vector.x, "y": chord_vector.y, "z": chord_vector.z}
                ],
                "section_model": section_model
            }

            wing_builders.append(virtual_end_sections)

        if self.virtual_end_disks[1]:
            virtual_end_sections = {
                "section_points": [
                    {"x": 0.0, "y": 0.0, "z": self.height},
                    {"x": 0.0, "y": 0.0, "z": self.height + self.virtual_end_disk_height_factor * self.diameter}
                ],
                "chord_vectors": [
                    {"x": chord_vector.x, "y": chord_vector.y, "z": chord_vector.z},
                    {"x": chord_vector.x, "y": chord_vector.y, "z": chord_vector.z}
                ],
                "section_model": section_model
            }

            wing_builders.append(virtual_end_sections)

        line_force_model = {
            "wing_builders": wing_builders,
            "nr_sections": self.nr_sections,
            "density": self.density
        }

        if self.smoothing_length is not None:
            end_corrections = [(False, False), (False, True)] if self.z_symmetry else [(True, False), (False, True)]

            gaussian_smoothing_settings = {
                "length_factor": self.smoothing_length,
                "end_corrections": end_corrections
            }

            smoothing_settings = {
                "gaussian": gaussian_smoothing_settings
            }

            line_force_model["smoothing_settings"] = smoothing_settings

        return line_force_model

    
    def run(self):
        freestream_velocity = SpatialVector(self.freestream_velocity, 0.0, 0.0)
        line_force_model = self.get_line_force_model()

        solver = {
            "SimpleIterative": {
                "damping_factor": 0.05,
                "max_iterations_per_time_step": 10,
            }
        }

        #"only_consider_change_in_angle": self.only_consider_change_in_angle

        end_time = 50 * self.diameter / self.freestream_velocity
        dt = end_time / 200

        wake = {}

        if self.z_symmetry:
            wake["symmetry_condition"] = "Z"

        match self.simulation_mode:
            case SimulationMode.DYNAMIC:
                wake["ratio_of_wake_affected_by_induced_velocities"] = 0.75
                wake["first_panel_relative_length"] = 0.75
                wake["last_panel_relative_length"] = 50.0
                wake["use_chord_direction"] = True

                wake["wake_length"] = {
                    "NrPanels": 60
                }

                wake["viscous_core_length_off_body"] = {
                    "Absolute": 0.25 * self.diameter
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
            "write_wake_data_to_file": self.write_wake_files,
            "wake_files_folder_path": "wake_files_output"
        }

        setup_string = json.dumps(setup, indent=4)

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

        while current_time < end_time:
            result = simulation.do_step(
                time = current_time, 
                time_step = dt, 
                freestream_velocity = freestream_velocity_list
            )

            current_time += dt

            result_history.append(result)

        return result_history