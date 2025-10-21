'''
Python logic for creating JSON setup files that act as input to the simulation. The are generated
by using parameters from dataclasses. This allows some flexibility in the setup of the simulation.
However, the logic is also kept simple to make it easy to understand. More variation in the setup
can be added.
'''

from collections import OrderedDict
from dataclasses import dataclass, field
import json

from stormbird_setup.direct_setup.spatial_vector import SpatialVector
from stormbird_setup.direct_setup.line_force_model import LineForceModelBuilder, WingBuilder
from stormbird_setup.direct_setup.section_models import SectionModel, Foil

from typing import Any

import numpy as np
import math

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
class LiftingLineSimulation:
    chord: float = 11.0
    span: float = 33.0
    start_height: float = 10.0
    stall_angle_deg: float = 40.0
    x_locations: list[float] = field(default_factory = lambda: [-60, 60])
    y_locations: list[float] = field(default_factory = lambda: [0.0, 0.0])
    nr_sections: int = 20
    write_wake_files: bool = False
    use_smoothing: bool = True

    @property
    def nr_sails(self) -> int:
        return len(self.x_locations)

    @property
    def z_locations(self) -> list[float]:
        return [self.start_height] * self.nr_sails

    def section_model(self) -> SectionModel:
        return SectionModel(
            model = Foil(
                mean_negative_stall_angle = math.radians(self.stall_angle_deg),
                mean_positive_stall_angle = math.radians(self.stall_angle_deg)
            )
        )

    @property
    def chord_vector(self) -> dict[str, Any]:
        return {"x": -self.chord, "y": 0.0, "z": 0.0}

    def line_force_model_builder(self) ->LineForceModelBuilder:
        line_force_model_builder = LineForceModelBuilder()

        for i in range(self.nr_sails):
            chord_vector = SpatialVector(x = -self.chord)

            wing_builder = WingBuilder(
                section_points = [
                    SpatialVector(
                        x = self.x_locations[i],
                        y = self.y_locations[i],
                        z = self.z_locations[i]
                    ),
                    SpatialVector(
                        x = self.x_locations[i],
                        y = self.y_locations[i],
                        z = self.z_locations[i] + self.span
                    )
                ],
                chord_vectors = [chord_vector, chord_vector],
                section_model = self.section_model(),
            )

            line_force_model_builder.add_wing_builder(wing_builder)

        return line_force_model_builder

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

    def to_json_file(self, file_path: str):
        with open(file_path, "w") as f:
            json.dump(self.to_dict(), f, indent=4)
