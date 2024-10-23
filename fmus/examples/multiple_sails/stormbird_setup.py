import numpy as np
from collections import OrderedDict

from dataclasses import dataclass

import json

model_scale_factor = 1.0

@dataclass
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



def make_sail_controller_setup_file():

    max_angle = 20.0
    wind_direction_data = np.array([-180, -20.0, -10.0, 0.0, 10.0, 20.0, 180])
    angle_of_attack_data = np.array([-max_angle, -max_angle, -max_angle, 0.0, max_angle, max_angle, max_angle])

    wing_angle_data = wind_direction_data + angle_of_attack_data

    nr_sails = 2

    controller = {
        "controllers": []
    }

    for _ in range(nr_sails):
        controller["controllers"].append(
            {
                "wind_direction_data": wind_direction_data.tolist(),
                "local_wing_angles_data": wing_angle_data.tolist(),
            }
        )

    with open("sail_controller_setup.json", "w") as f:
        json.dump(controller, f, indent=4)


def make_stormbird_setup_file():
    chord = 11.0 / model_scale_factor
    span = 33.0 / model_scale_factor

    start_height = 10.0 / model_scale_factor

    x_locations = np.array([-60, 60]) / model_scale_factor
    y_locations = np.array([0.0, 0.0]) / model_scale_factor
    z_locations = np.array([start_height, start_height, start_height])

    nr_sails = len(x_locations)

    out_dict = OrderedDict()

    chord_vector = {"x": -chord, "y": 0.0, "z": 0.0}
    section_model = {
        "Foil": {
            "cl_zero_angle": 0.0,
            "mean_positive_stall_angle": np.radians(45),
            "mean_negative_stall_angle": np.radians(45)
        }
    }
    non_zero_circulation_at_ends = [False, False]

    wing_builders = []

    for i in range(nr_sails):
        wing_builders.append(
            {
                "section_points": [
                    {"x": x_locations[i], "y": y_locations[i], "z": -z_locations[i]},
                    {"x": x_locations[i], "y": y_locations[i], "z": -(z_locations[i] + span)}
                ],
                "chord_vectors": [chord_vector, chord_vector],
                "section_model": section_model,
                "non_zero_circulation_at_ends": non_zero_circulation_at_ends
            }
        )

    out_dict["line_force_model"] = {
        "wing_builders": wing_builders,
        "nr_sections": 20
    }

    out_dict["simulation_mode"] = {
        "Dynamic": {
            "wake": {
                "wake_length": {
                    "NrPanels": 50
                },
                "ratio_of_wake_affected_by_induced_velocities": 0.0,
                "use_chord_direction": True,
                "symmetry_condition": "Z"
            }
        }
    }

    out_dict["write_wake_data_to_file"] = True
    out_dict["wake_files_folder_path"] = "wake_files"

    with open("stormbird_setup.json", "w") as f:
        json.dump(out_dict, f, indent=4)
