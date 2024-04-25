import json
import numpy as np

def get_line_force_model(
        chord: float = 2.0, 
        span: float = 10.0,
        angle_of_attack: float = 0.0,
        cl_zero_angle_of_attack: float = 0.0,
        cd_zero_angle_of_attack: float = 0.01,
        nr_sections: int = 16
    ) -> dict:
    chord_x = chord * np.cos(angle_of_attack)
    chord_y = -chord * np.sin(angle_of_attack)
    
    wing = {
        "section_points": [
            {"x": 0.0, "y": 0.0, "z": -span/2},
            {"x": 0.0, "y": 0.0, "z": span/2}
        ],
        "chord_vectors": [
            {"x": chord_x, "y": chord_y, "z": 0.0},
            {"x": chord_x, "y": chord_y, "z": 0.0}
        ],
        "section_model": {
            "Foil": {
                "cl_zero_angle_of_attack": cl_zero_angle_of_attack,
                "cd_zero_angle_of_attack": cd_zero_angle_of_attack
            }
        }
    }

    line_force_model = {
        "wing_builders": [wing],
        "nr_sections": nr_sections
    }

    return line_force_model

def get_sim_settings(
    sim_mode: str = "QuasiSteady",
) -> dict:
    if sim_mode == "QuasiSteady":
        return {
            "QuasiSteady": {}
        }
    elif sim_mode == "Dynamic":
        return {
            "Dynamic": {}
        }
    else:
        raise ValueError(f'Unknown sim_mode: {sim_mode}. Available options are "QuasiSteady" and "Dynamic"')

        