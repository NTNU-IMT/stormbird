
from stormbird_setup.direct_setup.lifting_line.simulation_builder import SimulationBuilder
from stormbird_setup.direct_setup.lifting_line.complete_sail_model import CompleteSailModelBuilder
from stormbird_setup.direct_setup.controller import ControllerBuilder

from stormbird_setup.simplified_setup.simple_sail_setup import SimpleSailSetup, SailType
from stormbird_setup.direct_setup.spatial_vector import SpatialVector
from stormbird_setup.direct_setup.wind import WindEnvironment

from pystormbird.lifting_line import CompleteSailModel

import numpy as np
import matplotlib.pyplot as plt

if __name__ == "__main__":
    sail_types_to_compare = [
        SailType.WingSailSingleElement,
        SailType.WingSailTwoElement,
        SailType.RotorSail,
        SailType.SuctionSail
    ]

    default_colors = plt.rcParams['axes.prop_cycle'].by_key()['color']

    chord_length = 5.0
    height = 35.0
    area = chord_length * height
    deck_height = 10.0

    ship_velocity = 12.0 * 0.5144444
    wind_velocity = 8.0
    density = 1.225
    wind_directions_deg = np.arange(-180.0, 181, 2)

    w_plot = 16
    h_plot = w_plot / 2.35
    fig = plt.figure(figsize=(w_plot, h_plot))
    ax_power = fig.add_subplot(1, 2, 1)
    ax_control = fig.add_subplot(1, 2, 2)

    for sail_index, sail_type in enumerate(sail_types_to_compare):
        simulation_builder = SimulationBuilder()

        sail = SimpleSailSetup(
            position = SpatialVector(x=0.0, y=0.0, z=deck_height),
            chord_length = chord_length,
            height = height,
            sail_type = sail_type
        )

        simulation_builder.line_force_model.add_wing_builder(sail.wing_builder())

        controller_set_points = sail.controller_set_points()

        wind_environment = WindEnvironment(
            height_variation_model=None
        )

        model_builder = CompleteSailModelBuilder(
            lifting_line_simulation=simulation_builder,
            controller=ControllerBuilder(set_points = [controller_set_points]),
            wind_environment=wind_environment,
        )

        model = CompleteSailModel(model_builder.to_json_string())

        thrust = np.zeros_like(wind_directions_deg)
        propulsive_power = np.zeros_like(wind_directions_deg)
        power_net = np.zeros_like(wind_directions_deg)
        apparent_wind_direction = np.zeros_like(wind_directions_deg)

        section_model_internal_state = np.zeros_like(wind_directions_deg)

        for index, wind_dir_deg in enumerate(wind_directions_deg):
            wind_dir_rad = np.radians(wind_dir_deg)

            u_wind_apparent = ship_velocity + wind_velocity * np.cos(wind_dir_rad)
            v_wind_apparent = -wind_velocity * np.sin(wind_dir_rad)

            u_inf = np.sqrt(u_wind_apparent**2 + v_wind_apparent**2)

            apparent_wind_direction[index] = np.arctan2(-v_wind_apparent, u_wind_apparent)

            result = model.simulate_condition(
                wind_velocity = wind_velocity,
                wind_direction = wind_dir_rad,
                ship_velocity = ship_velocity,
            )

            thrust[index] = -result.integrated_forces_sum()[0]

            propulsive_power[index] = thrust[index] * ship_velocity
            power_net[index] = propulsive_power[index] - result.input_power_sum()

            section_model_internal_state[index] = model.section_models_internal_state()[0]

        ax_power.plot(
            wind_directions_deg,
            power_net / area,
            label=f"{sail_type.value} effective",
            color=default_colors[sail_index]
        )

        if sail.sail_type.consumes_power():
            ax_power.plot(
                wind_directions_deg,
                propulsive_power / area,
                linestyle='--',
                label=f"{sail_type.value} propulsive",
                color=default_colors[sail_index]
            )

        ax_control.plot(
            wind_directions_deg,
            section_model_internal_state,
            label=sail_type.value,
            color=default_colors[sail_index]
        )

    ax_power.set_xlabel("Wind direction (deg)")
    ax_control.set_xlabel("Wind direction (deg)")

    ax_power.set_ylabel("Power per area (W/m^2)")
    ax_control.set_ylabel("Control Surface Setting (rad or spin ratio)")

    ax_power.legend()

    ax_power.set_title(
        f"Ship velocity: {ship_velocity/0.5144444:.1f} kn, Wind velocity: {wind_velocity:.1f} m/s"
    )

    plt.show()
