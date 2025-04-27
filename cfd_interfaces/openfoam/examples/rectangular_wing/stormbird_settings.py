
import json
import numpy as np

from dataclasses import dataclass

@dataclass(frozen=True, kw_only=True)
class StormbirdSettings():
    angle_of_attack_deg: float
    velocity: float = 1.0
    chord_length: float = 1.0
    span: float = 4.0

    @property
    def force_factor(self) -> float:
        return 0.5 * self.chord_length * self.span * self.velocity**2
    
    def get_line_force_model_dict(self):
        chord_vector = {
            "x": self.chord_length * np.cos(np.radians(self.angle_of_attack_deg)),
            "y": -self.chord_length * np.sin(np.radians(self.angle_of_attack_deg)),
            "z": 0.0
        }
        
        wing_builder = {
            "section_points": [
                {"x": 0.0, "y": 0.0, "z": 0.0},
                {"x": 0.0, "y": 0.0, "z": self.span}
            ],
            "chord_vectors": [chord_vector, chord_vector],
            "section_model": {
                "Foil": {
                    "cl_zero_angle": 0.5,
                }
            },
            "non_zero_circulation_at_ends": [False, False]
        }

        line_force_model = {
            "wing_builders": [wing_builder],
            "nr_sections": 64,
            "density": 1.0
        }

        return line_force_model
    
    def get_actuator_line_setup_dict(self):
        line_force_model = self.get_line_force_model_dict()

        projection = {
            "Gaussian": {
                "chord_factor": 0.4,
                "thickness_factor": 0.1
            }
        }

        return {
            "line_force_model": line_force_model,
            "projection": projection,
            "solver_settings": {
                "strength_damping": 0.1
            }
        }
    
    def write_actuator_line_setup_to_file(self, file_path):
        setup_dict = self.get_actuator_line_setup_dict()
        
        with open(file_path, 'w') as f:
            json.dump(setup_dict, f, indent=4)

    def write_dimensions(self, file_path):
        with open(file_path, 'w') as f:
            f.write(f"chord {self.chord_length};\n")
            f.write(f"span {self.span};\n")
            f.write(f"angle_of_attack_deg {self.angle_of_attack_deg};\n")