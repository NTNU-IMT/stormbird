'''
Python logic for creating JSON setup files that act as input to the simulation. The are generated
by using parameters from dataclasses. This allows some flexibility in the setup of the simulation.
However, the logic is also kept simple to make it easy to understand. More variation in the setup
can be added.
'''

from collections import OrderedDict
from dataclasses import dataclass, field
import json

from typing import Any

import numpy as np

@dataclass(kw_only=True)
class WindEnvironment:
    reference_height: float = 10.0
    power_factor: float = 1.0 / 9.0

    def to_dict(self):
        return {
            "PowerModel": {
                "reference_height": self.reference_height,
                "power_factor": self.power_factor
            }
        }

    def to_json_file(self, file_path):
        with open(file_path, "w") as f:
            json.dump(self.to_dict(), f, indent=4)


@dataclass(kw_only=True)
class SailController:
    target_angle: float = -10.0
    max_rotational_speed: float = 10.0
    nr_sails: int = 2
    initial_wing_angle: float = 120.0
    angles_in_degrees: bool = True
    moving_average_window_size: int = 2
    update_factor: float = 0.2

    def to_dict(self):
        target_angles_of_attack = [self.target_angle] * self.nr_sails
        initial_wing_angles = [self.initial_wing_angle] * self.nr_sails

        return {
            "target_angles_of_attack": target_angles_of_attack,
            "max_rotational_speed": self.max_rotational_speed,
            "angles_in_degrees": self.angles_in_degrees,
            "initial_wing_angles": initial_wing_angles,
            "moving_average_window_size": self.moving_average_window_size,
            "update_factor": self.update_factor
        }

    def to_json_file(self, file_path):
        with open(file_path, "w") as f:
            json.dump(self.to_dict(), f, indent=4)

@dataclass(kw_only=True)
class LiftingLineSimulation:
    chord: float = 11.0
    span: float = 33.0
    start_height: float = 10.0
    stall_angle_deg: float = 40.0
    x_locations: list = field(default_factory = lambda: [-60, 60])
    y_locations: list = field(default_factory = lambda: [0.0, 0.0])
    nr_sections: int = 20
    write_wake_files: bool = False
    use_smoothing: bool = True

    @property
    def nr_sails(self) -> int:
        return len(self.x_locations)

    @property
    def z_locations(self) -> list[float]:
        return [self.start_height] * self.nr_sails

    @property
    def section_model(self) -> dict[str, Any]:
        return {
            "Foil": {
                "cl_zero_angle": 0.0,
                "mean_positive_stall_angle": np.radians(self.stall_angle_deg),
                "mean_negative_stall_angle": np.radians(self.stall_angle_deg),
            }
        }

    @property
    def chord_vector(self) -> dict[str, Any]:
        return {"x": -self.chord, "y": 0.0, "z": 0.0}

    @property
    def non_zero_circulation_at_ends(self):
        return [False, False]

    def line_force_model_dict(self) -> dict[str, Any]:
        out_dict = OrderedDict()

        wing_builders = []

        for i in range(self.nr_sails):
            wing_builders.append(
                {
                    "section_points": [
                        {"x": self.x_locations[i], "y": self.y_locations[i], "z": -self.z_locations[i]},
                        {"x": self.x_locations[i], "y": self.y_locations[i], "z": -(self.z_locations[i] + self.span)}
                    ],
                    "chord_vectors": [self.chord_vector, self.chord_vector],
                    "section_model": self.section_model,
                    "non_zero_circulation_at_ends": self.non_zero_circulation_at_ends
                }
            )

        out_dict["wing_builders"] = wing_builders
        out_dict["nr_sections"] = self.nr_sections
        out_dict["output_coordinate_system"] = "Body"

        if self.use_smoothing:
            gaussian_smoothing = {
                "length_factor": 0.1,
                "end_corrections_delta_span_factor": 1.0,
                "end_corrections_number_of_insertions": 3
            }

            out_dict["circulation_corrections"] = {
                "GaussianSmoothing": gaussian_smoothing
            }

        return out_dict

    def wake_settings_dict(self) -> dict[str, Any]:
        return {
            "wake_length": {
                "NrPanels": 100
            },
            "last_panel_relative_length": 5.0,
            "ratio_of_wake_affected_by_induced_velocities": 0.0,
            "use_chord_direction": True,
            "symmetry_condition": "Z",
            "strength_damping": "DirectFromStall"
        }

    def solver_settings_dict(self) -> dict[str, Any]:
        return {
            "max_iterations_per_time_step": 10,
            "damping_factor": 0.05,
        }

    def to_dict(self) -> dict[str, Any]:
        out_dict = OrderedDict()

        out_dict["line_force_model"] = self.line_force_model_dict()

        out_dict["simulation_mode"] = {
            "Dynamic": {
                "wake": self.wake_settings_dict(),
                "solver": self.solver_settings_dict()
            }
        }

        out_dict["write_wake_data_to_file"] = self.write_wake_files
        out_dict["wake_files_folder_path"] = "wake_files"

        return out_dict

    def to_json_file(self, file_path):
        with open(file_path, "w") as f:
            json.dump(self.to_dict(), f, indent=4)
