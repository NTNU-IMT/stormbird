import numpy as np
from collections import OrderedDict

import json


def make_stormbird_setup_file():
    model_scale_factor = 32.0
    
    chord = 11.0 / model_scale_factor
    span = 33.0 / model_scale_factor

    x_locations = np.array([-60, 0.0, 60]) / model_scale_factor
    y_locations = np.array([0.0, 0.0, 0.0]) / model_scale_factor
    z_locations = np.array([0.0, 0.0, 0.0]) / model_scale_factor

    nr_sails = len(x_locations)

    out_dict = OrderedDict()

    chord_vector = {"x": chord, "y": 0.0, "z": 0.0}
    section_model = {"Foil": {"cl_zero_angle": 0.0}}
    non_zero_circulation_at_ends = [False, False]

    wing_builders = []

    for i in range(nr_sails):
        wing_builders.append(
            {
                "section_points": [
                    {"x": x_locations[i], "y": y_locations[i], "z": z_locations[i]},
                    {"x": x_locations[i], "y": y_locations[i], "z": z_locations[i] + span}
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
                "ratio_of_wake_affected_by_induced_velocities": 0.1,
                "use_chord_direction": True
            }
        }
    }

    out_dict["write_wake_data_to_file"] = True
    out_dict["wake_files_folder_path"] = "wake_files"

    with open("stormbird_setup.json", "w") as f:
        json.dump(out_dict, f, indent=4)
