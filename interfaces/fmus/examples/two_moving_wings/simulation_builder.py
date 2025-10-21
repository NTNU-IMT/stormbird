'''
Python logic for creating JSON setup files that act as input to the simulation. The are generated
by using parameters from dataclasses. This allows some flexibility in the setup of the simulation.
However, the logic is also kept simple to make it easy to understand. More variation in the setup
can be added.
'''

from dataclasses import dataclass, field

from stormbird_setup.direct_setup.spatial_vector import SpatialVector
from stormbird_setup.direct_setup.line_force_model import LineForceModelBuilder, WingBuilder
from stormbird_setup.direct_setup.circulation_corrections import CirculationCorrectionBuilder
from stormbird_setup.direct_setup.section_models import SectionModel, Foil
from stormbird_setup.direct_setup.lifting_line.simulation_builder import SimulationBuilder, DynamicSettings


import math

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

        line_force_model_builder.nr_sections = self.nr_sections

        line_force_model_builder.circulation_correction = CirculationCorrectionBuilder.new_gaussian_smoothing()

        return line_force_model_builder

    def simulation_builder(self) -> SimulationBuilder:
        simulation_builder = SimulationBuilder(
            line_force_model = self.line_force_model_builder(),
            simulation_settings = DynamicSettings()
        )

        simulation_builder.simulation_settings.wake.write_wake_data_to_file = self.write_wake_files
        simulation_builder.simulation_settings.wake.wake_files_folder_path = "wake_files"

        return simulation_builder
