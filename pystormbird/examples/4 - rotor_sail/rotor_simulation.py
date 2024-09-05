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
    nr_sections: int = 32
    smoothing_length: float | None = None
    simulation_mode: SimulationMode = SimulationMode.STATIC
    smoothing_length: float | None = None
    z_symmetry: bool = True
    virtual_end_disks: tuple[bool, bool] = (False, False)
    virtual_end_disk_height_factor: float = 0.5
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

        if self.virtual_end_disks[0] and self.virtual_end_disks[1]:
            non_zero_circulation_at_ends = [True, True]
        elif self.virtual_end_disks[0]:
            non_zero_circulation_at_ends = [True, False]
        elif self.virtual_end_disks[1]:
            non_zero_circulation_at_ends = [False, True]
        else:
            non_zero_circulation_at_ends = [False, False]
        
        rotor_builder = {
            "section_points": [
                {"x": 0.0, "y": 0.0, "z": 0.0},
                {"x": 0.0, "y": 0.0, "z": self.height}
            ],
            "chord_vectors": [
                {"x": chord_vector.x, "y": chord_vector.y, "z": chord_vector.z},
                {"x": chord_vector.x, "y": chord_vector.y, "z": chord_vector.z}
            ],
            "section_model": section_model,
            "non_zero_circulation_at_ends": non_zero_circulation_at_ends
        }

        wing_builders = [rotor_builder]

        span_virtual_end_disks = self.virtual_end_disk_height_factor * self.diameter

        nr_sections_virtual_end_disks = int(self.nr_sections * span_virtual_end_disks / self.height)

        nr_sections_virtual_end_disks = max(2, nr_sections_virtual_end_disks)

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
                "section_model": section_model,
                "non_zero_circulation_at_ends": [False, True],
                "nr_sections": nr_sections_virtual_end_disks
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
                "section_model": section_model,
                "non_zero_circulation_at_ends": [True, False],
                "nr_sections": nr_sections_virtual_end_disks
            }

            wing_builders.append(virtual_end_sections)

        line_force_model = {
            "wing_builders": wing_builders,
            "nr_sections": self.nr_sections,
            "density": self.density
        }

        if self.smoothing_length is not None:
            gaussian_smoothing = {
                "length_factor": self.smoothing_length,
            }

            line_force_model["circulation_corrections"] = {
                "GaussianSmoothing": gaussian_smoothing
            }

        return line_force_model

    
    def run(self):
        freestream_velocity = SpatialVector(self.freestream_velocity, 0.0, 0.0)
        line_force_model = self.get_line_force_model()

        wake = {}
        solver = {
            "velocity_corrections": {
                "MaxInducedVelocityMagnitudeRatio": 1.0
            }#"FixedMagnitudeEqualToFreestream"
        }

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

        setup_string = json.dumps(setup, indent=4)

        simulation = Simulation(
            setup_string = setup_string,
            initial_time_step = 1,
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

        if self.simulation_mode == SimulationMode.DYNAMIC:
            end_time = 20 * self.diameter / self.freestream_velocity
            dt = end_time / 200

            while current_time < end_time:
                result = simulation.do_step(
                    time = current_time, 
                    time_step = dt, 
                    freestream_velocity = freestream_velocity_list
                )

                current_time += dt

                result_history.append(result)
        else:
            result = simulation.do_step(
                time = current_time, 
                time_step = 1, 
                freestream_velocity = freestream_velocity_list
            )

            result_history.append(result)

        print("Last number of iterations: ", result_history[-1].iterations)

        return result_history