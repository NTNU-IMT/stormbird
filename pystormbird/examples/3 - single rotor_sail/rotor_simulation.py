import json
import numpy as np

from pystormbird.lifting_line import Simulation
from pystormbird import SpatialVector

from dataclasses import dataclass
from enum import Enum

class SimulationMode(Enum):
    DYNAMIC = 0
    STATIC = 1

class TestCase(Enum):
    RAW_SIMULATION = 0
    FIXED_VELOCITY_MAGNITUDE = 1
    LIMIT_ON_INDUCED_VELOCITY = 2
    VIRTUAL_END_DISK = 3

    def to_string(self):
        return self.name.replace("_", " ").lower()

    @property
    def virtual_end_disk(self) -> bool:
        match self:
            case TestCase.VIRTUAL_END_DISK:
                return True
            case _:
                return False
            
    def non_zero_circulation_at_ends(self, z_symmetry: bool) -> tuple[bool, bool]:
        match self:
            case TestCase.VIRTUAL_END_DISK:
                if z_symmetry:
                    return (True, True)
                else:
                    return (False, True)
            case _:
                if z_symmetry:
                    return (True, False)
                else:
                    return (False, False)
            
    @property
    def velocity_corrections(self) -> dict | None:
        match self:
            case TestCase.FIXED_VELOCITY_MAGNITUDE:
                return "FixedMagnitudeEqualToFreestream"
            case TestCase.LIMIT_ON_INDUCED_VELOCITY:
                return {
                    "MaxInducedVelocityMagnitudeRatio": 1.0
                }
            case _:
                return None


@dataclass(frozen=True, kw_only=True)
class RotorSimulationCase():
    spin_ratio: float
    diameter: float = 5.0
    height: float = 30.0
    freestream_velocity: float = 8.0
    density: float = 1.225
    nr_sections: int = 32
    simulation_mode: SimulationMode = SimulationMode.STATIC
    z_symmetry: bool = True
    test_case: TestCase = TestCase.RAW_SIMULATION
    virtual_end_disk_height_factor: float = 0.35
    write_wake_files: bool = False

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
            "non_zero_circulation_at_ends": self.test_case.non_zero_circulation_at_ends(self.z_symmetry)
        }

        wing_builders = [rotor_builder]

        if self.test_case.virtual_end_disk:
            span_virtual_end_disks = self.virtual_end_disk_height_factor * self.diameter
            nr_sections_virtual_end_disks = int(self.nr_sections * span_virtual_end_disks / self.height)

            nr_sections_virtual_end_disks = max(4, nr_sections_virtual_end_disks)
            
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

        return line_force_model

    
    def run(self):
        freestream_velocity = SpatialVector(self.freestream_velocity, 0.0, 0.0)
        line_force_model = self.get_line_force_model()

        wake = {}

        velocity_corrections = self.test_case.velocity_corrections

        if velocity_corrections is not None:
            solver = {
                "velocity_corrections": velocity_corrections
            }
        else:
            solver = {}

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