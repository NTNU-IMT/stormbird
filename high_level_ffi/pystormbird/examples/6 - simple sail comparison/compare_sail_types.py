
from stormbird_setup.lifting_line.simulation_builder import SimulationBuilder
from stormbird_setup.lifting_line.complete_sail_model import CompleteSailModelBuilder
from stormbird_setup.lifting_line.solver import SimpleIterative

from stormbird_setup.simple_sail_setup import SimpleSailSetup, SailType
from stormbird_setup.spatial_vector import SpatialVector

from pystormbird.lifting_line import CompleteSailModel

import numpy as np
import matplotlib.pyplot as plt

if __name__ == "__main__":
    sail_types_to_compare = [
        SailType.WingSailSingleElement,
        SailType.WingSailFlapped,
        SailType.RotorSail
    ]

    chord_length = 5.0
    height = 35.0
    area = chord_length * height
    deck_height = 10.0

    ship_velocity = 12.0 * 0.5144444
    wind_velocity = 8.0
    density = 1.225
    wind_directions_deg = np.linspace(-180, 180, 90)

    for sail_type in sail_types_to_compare:
        simulation_builder = SimulationBuilder()

        sail = SimpleSailSetup(
            position = SpatialVector(x=0.0, y=0.0, z=deck_height),
            chord_length = chord_length,
            height = height,
            sail_type = sail_type
        )

        simulation_builder.line_force_model.add_wing_builder(sail.wing_builder())

        controller = sail.controller_builder()

        model_builder = CompleteSailModelBuilder(
            lifting_line_simulation=simulation_builder,
            controller=controller
        )

        if sail_type == SailType.RotorSail:
            model_builder.lifting_line_simulation.simulation_settings.solver = SimpleIterative(
                max_iterations_per_time_step = 200,
                damping_factor = 0.1
            )

        model = CompleteSailModel(model_builder.to_json_string())

        thrust = np.zeros_like(wind_directions_deg)
        apparent_wind_direction = np.zeros_like(wind_directions_deg)

        for index, wind_dir_deg in enumerate(wind_directions_deg):
            wind_dir_rad = np.radians(wind_dir_deg)

            u_wind_apparent = ship_velocity + wind_velocity * np.cos(wind_dir_rad)
            v_wind_apparent = wind_velocity * np.sin(wind_dir_rad)

            u_inf = np.sqrt(u_wind_apparent**2 + v_wind_apparent**2)

            apparent_wind_direction[index] = np.arctan2(v_wind_apparent, u_wind_apparent)

            result = model.simulate_condition(
                wind_velocity = wind_velocity,
                wind_direction = wind_dir_rad,
                ship_velocity = ship_velocity,
            )

            thrust[index] = -result.integrated_forces_sum().x / (0.5 * density * area * u_inf**2)

        plt.plot(np.degrees(apparent_wind_direction), thrust, label=sail_type.value)

    plt.show()

            