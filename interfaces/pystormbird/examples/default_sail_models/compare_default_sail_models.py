"""
This example shows how to use the default sail models in the stormbird_setup library. The library
comes with a limited set of sail models, which are either best guesses for generic sails, or tuned
to specific sources in the literature. Each model contains a section model, controller set points
and solver settings, and can be scaled to different sail sizes. Note, the point of the default
models is not necessarily to be very accurate compared to any specific supplier, as all sail types
have large variations in performance parameters. However, the settings are intended to be
reasonable values, useful if one wants to make a quick comparison, for whatever reason.

The example sets up a complete sail model for each default sail, with the same size, and compares
the lift and drag, and the thrust and power when the sails are operated at the optimal loading.
"""

from stormbird_setup.default_sails import DefaultSail
from stormbird_setup.lifting_line import CompleteSailModelBuilder, SymmetryCondition
from stormbird_setup.section_models import RotatingCylinder, ActiveAirFlowFoil
from stormbird_setup.spatial_vector import SpatialVector
from stormbird_setup.utils import revolutions_per_second_from_spin_ratio

from pystormbird.lifting_line import CompleteSailModel
from pystormbird.wind import WindCondition

import numpy as np
import matplotlib.pyplot as plt

def wing_internal_state_at_max_set_point(
    model: CompleteSailModel,
    wind_velocity: float
) -> float:
    '''
    Uses the controller to find the wing internal state at the maximum set points, for a wind from
    the side. For many section models, the controller converts the set points to the wing internal
    state, such as from the suction coefficient to the fan power for a suction sail, which is why
    the controller is used. The internal state set points of the default sails have the same sign
    as the angle of attack set points, so the absolute value is returned, to be used together with
    positive angles of attack.
    '''
    model.apply_controller_based_on_wind_condition(
        time = 0,
        time_step = 1,
        wind_condition = WindCondition.new_constant(
            direction_coming_from = np.radians(90.0),
            velocity = wind_velocity
        ),
        ship_velocity = 0.0,
        controller_loading = 1.0
    )

    return abs(model.internal_state()[0])

if __name__ == "__main__":
    default_sails_to_compare = [
        DefaultSail.GenericSingleElementWingSail,
        DefaultSail.GenericTwoElementWingSail,
        DefaultSail.RotorSailDeybach2024,
        DefaultSail.SuctionSailCharrier1985
    ]

    default_colors = plt.rcParams['axes.prop_cycle'].by_key()['color']

    chord_length = 5.0
    height = 35.0
    area = chord_length * height
    deck_height = 0.0

    ship_velocity = 12.0 * 0.5144444
    wind_velocity = 8.0
    density = 1.225
    wind_directions_deg = np.arange(-180.0, 181, 4)

    fig, axes = plt.subplots(2, 2, figsize = (10, 10))

    ax_cl = axes[0, 0]
    ax_cd = axes[0, 1]
    ax_thrust = axes[1, 0]
    ax_power = axes[1, 1]

    # The lift of the rotor sail is plotted as a function of the spin ratio, on a separate x-axis
    ax_cl_spin_ratio = ax_cl.twiny()

    for sail_index, default_sail in enumerate(default_sails_to_compare):
        model_builder = CompleteSailModelBuilder.new_from_default_sail(
            sail = default_sail,
            positions = [SpatialVector(x=0.0, y=0.0, z=deck_height)],
            chord_length = chord_length,
            height = height,
            symmetry_condition = SymmetryCondition.Z,
        )

        model = CompleteSailModel(model_builder.to_json_string())

        wind_environment = model.get_wind_environment()

        section_model = default_sail.section_model().model
        controller_set_points = model_builder.controller.set_points[0]

        is_rotor_sail = isinstance(section_model, RotatingCylinder)
        consumes_power = isinstance(section_model, (RotatingCylinder, ActiveAirFlowFoil))

        # --------------- Lift and drag  --------------------------------------------------
        n_test = 40

        if is_rotor_sail:
            max_spin_ratio = np.max(controller_set_points.internal_state_data)

            spin_ratio_test = np.linspace(0, 5.0, n_test)
        else:
            max_angle_of_attack = np.max(controller_set_points.angle_of_attack_data)

            if controller_set_points.internal_state_data is not None:
                model.set_internal_state(
                    [wing_internal_state_at_max_set_point(model, wind_velocity)]
                )

            alpha_test = np.radians(np.linspace(0, 35, n_test))

        cl = np.zeros(n_test)
        cd = np.zeros(n_test)

        wind_condition = WindCondition.new_constant(
            direction_coming_from = 0.0,
            velocity = wind_velocity
        )

        for i_test in range(n_test):
            if is_rotor_sail:
                rps = revolutions_per_second_from_spin_ratio(
                    spin_ratio = spin_ratio_test[i_test],
                    diameter = chord_length,
                    velocity = wind_velocity
                )

                model.set_internal_state([rps])
            else:
                model.set_local_wing_angles([-alpha_test[i_test]])

            result = model.do_step(
                time = 0,
                time_step = 1,
                wind_condition = wind_condition,
                ship_velocity = 0.0
            )

            forces = result.integrated_forces_sum()

            cd[i_test] = forces[0] / (0.5 * density * area * wind_velocity**2)
            cl[i_test] = forces[1] / (0.5 * density * area * wind_velocity**2)

        color = default_colors[sail_index]

        if is_rotor_sail:
            ax_cl_spin_ratio.plot(spin_ratio_test, cl, color = color)
            ax_cl_spin_ratio.plot(
                max_spin_ratio, np.interp(max_spin_ratio, spin_ratio_test, cl), "o", color = color
            )
        else:
            ax_cl.plot(np.degrees(alpha_test), cl, color = color)
            ax_cl.plot(
                np.degrees(max_angle_of_attack),
                np.interp(max_angle_of_attack, alpha_test, cl),
                "o",
                color = color
            )

        ax_cd.plot(cl, cd, color = color)

        # --------------- Compute thrust and net power ------------------------------------
        thrust_coefficient = np.zeros_like(wind_directions_deg)
        propulsive_power = np.zeros_like(wind_directions_deg)
        power_net = np.zeros_like(wind_directions_deg)
        apparent_wind_direction = np.zeros_like(wind_directions_deg)

        for index, wind_dir_deg in enumerate(wind_directions_deg):
            wind_dir_rad = np.radians(wind_dir_deg)

            u_wind_apparent = ship_velocity + wind_velocity * np.cos(wind_dir_rad)
            v_wind_apparent = -wind_velocity * np.sin(wind_dir_rad)

            u_inf = np.sqrt(u_wind_apparent**2 + v_wind_apparent**2)

            wind_condition = WindCondition.new_constant(
                direction_coming_from = wind_dir_rad,
                velocity = wind_velocity
            )

            apparent_wind_direction[index] = wind_environment.apparent_wind_direction_from_condition_and_linear_velocity(
                condition = wind_condition,
                linear_velocity = [ship_velocity, 0.0, 0.0]
            )

            # The model tests different controller loadings, and returns the result with the
            # highest net power. The thrust is in the negative x-direction.
            results = model.simulate_optimal_steady_state_condition_simple_output(
                wind_condition = wind_condition,
                ship_velocity = ship_velocity,
                max_loading = 1.0
            )

            thrust = sum(-result.force[0] for result in results)
            input_power = sum(result.input_power for result in results)

            thrust_coefficient[index] = thrust / (0.5 * density * area * u_inf**2)
            propulsive_power[index] = thrust * ship_velocity
            power_net[index] = propulsive_power[index] - input_power

        if consumes_power:
            name = f"{default_sail.value} effective"
        else:
            name = f"{default_sail.value}"

        ax_thrust.plot(np.degrees(apparent_wind_direction), thrust_coefficient, color = color, label = name)
        ax_power.plot(np.degrees(apparent_wind_direction), power_net / area, color = color)

        if consumes_power:
            ax_power.plot(
                np.degrees(apparent_wind_direction),
                propulsive_power / area,
                "--",
                color = color
            )
            ax_thrust.plot([], [], "--", color = color, label = f"{default_sail.value} propulsive")

    ax_thrust.plot([], [], "o", color = "grey", label = "Max controller set point")

    ax_cl.set_xlabel("Angle of attack [deg]")
    ax_cl.set_ylabel("Lift coefficient")
    ax_cl_spin_ratio.set_xlabel("Spin ratio")

    ax_cd.set_xlabel("Lift coefficient")
    ax_cd.set_ylabel("Drag coefficient")

    ax_thrust.set_xlabel("Apparent wind direction [deg]")
    ax_thrust.set_ylabel("Thrust coefficient")
    ax_thrust.set_xlim(0, 180)

    ax_power.set_xlabel("Apparent wind direction [deg]")
    ax_power.set_ylabel("Power per area [W/m^2]")
    ax_power.set_xlim(0, 180)

    ship_velocity_knots = ship_velocity / 0.5144444
    ax_power.set_title(
        f"$V_{{wind}}$ = {wind_velocity:.1f} m/s, $V_{{ship}}$ = {ship_velocity_knots:.0f} kn",
        fontsize = 10
    )

    fig.legend(loc = "lower center", ncol = 3)

    plt.tight_layout(rect = (0, 0.08, 1, 1))

    plt.savefig("compare_default_sail_models.pdf")
    plt.savefig("compare_default_sail_models.svg")
    plt.savefig("compare_default_sail_models.png", dpi = 200)

    plt.show()
