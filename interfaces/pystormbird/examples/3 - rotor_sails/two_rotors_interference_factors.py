"""
This example compares the interference factors for two rotor sails, computed with a lifting line
model, against wind tunnel experiments. One rotor is placed at different angular positions around
the other, and the interference factors are the lift and drag on the first rotor divided by the
lift and drag on the same rotor when it stands alone.

The main point is to show the importance of the viscous core length evolution in the dynamic
wake. Real vortices have a viscous core that grows when moving away from the rotor. This is a very
important effect when one rotor stands directly in the wake of another, which happens when the
interfering rotor is upstream, at angular positions close to 0 degrees.

Three variants are compared:
- A quasi-steady simulation, where the wake does not have any viscous core length evolution
- Two dynamic simulations, with different values of the evolution length factor for the viscous
  core length. The factor sets how far into the wake the viscous core reaches its final value, as a
  fraction of the wake length. A lower value means that the core grows faster.

The dynamic simulations use a steady state strength update and no dynamic wake shape, so that the
only difference from the quasi-steady simulation is the viscous core length evolution.
"""

import numpy as np
import matplotlib.pyplot as plt
from stormbird_setup.spatial_vector import SpatialVector

import polars as pl

from setup import simulate_single_case, SolverType

# The variants of the simulation that are compared. Each variant gets its own line style in the plot
VARIANTS = [
    {
        "label": "Quasi-steady",
        "dynamic": False,
        "line_style": "-",
    },
    {
        "label": "Dynamic, evolution length factor 1.0",
        "dynamic": True,
        "evolution_length_factor": 1.0,
        "line_style": "--",
    },
    {
        "label": "Dynamic, evolution length factor 0.3",
        "dynamic": True,
        "evolution_length_factor": 0.3,
        "line_style": ":",
    },
]

if __name__ == "__main__":
    default_colors = plt.rcParams['axes.prop_cycle'].by_key()['color']

    exp_data = pl.read_csv("data/deybach_2026_interference_factors.csv", separator=";")

    spin_ratio = 2.5

    diameter = 5.0
    height = 35.0
    foundation_height = 1.875

    spacing = 4.0 * diameter

    rotations_deg = exp_data["Angular position of the interfering rotor [deg]"].to_numpy()
    n_rotations = len(rotations_deg)

    wind_direction_deg = 0.0

    solver = SolverType.Iterative

    w_plot = 16
    h_plot = w_plot / 1.85
    fig = plt.figure(figsize=(w_plot, h_plot))

    ax = fig.add_subplot(111)

    for variant in VARIANTS:
        print()
        print(variant["label"])

        simulation_settings = dict(
            diameter = diameter,
            height = height,
            foundation_height = foundation_height,
            spin_ratio = spin_ratio,
            solver_type = solver,
            wind_direction_deg = wind_direction_deg,
            dynamic = variant["dynamic"],
        )

        if variant["dynamic"]:
            simulation_settings["dynamic_shape"] = False
            simulation_settings["steady_state_strength_update"] = True
            simulation_settings["evolution_length_factor"] = variant["evolution_length_factor"]

        # The single rotor does not depend on the position of the interfering rotor, and is
        # therefore only simulated once for each variant
        res_single = simulate_single_case(
            rotor_x_locations = [0.0],
            rotor_y_locations = [0.0],
            **simulation_settings
        )

        cd_single = res_single[0]['cx']
        cl_single = res_single[0]['cy']

        cd = np.zeros(n_rotations)
        cl = np.zeros(n_rotations)

        for dir_index in range(n_rotations):
            print("Testing rotation: ", rotations_deg[dir_index])

            interfering_rotor_location = SpatialVector(x=-spacing).rotate_around_axis(
                np.radians(rotations_deg[dir_index]),
                axis=SpatialVector(z=1.0)
            )

            res = simulate_single_case(
                rotor_x_locations = [0.0, interfering_rotor_location.x],
                rotor_y_locations = [0.0, interfering_rotor_location.y],
                **simulation_settings
            )

            cd[dir_index] = res[0]['cx']
            cl[dir_index] = res[0]['cy']

        ax.plot(
            rotations_deg, cd / cd_single,
            variant["line_style"],
            color=default_colors[0],
            label="CD interference, " + variant["label"]
        )

        ax.plot(
            rotations_deg, cl / cl_single,
            variant["line_style"],
            color=default_colors[1],
            label="CL interference, " + variant["label"]
        )

    ax.scatter(
        rotations_deg,
        exp_data["IFD"].to_numpy(),
        color=default_colors[0],
        label="CD interference, experiment, Deybach et al. (2026)"
    )

    ax.scatter(
        rotations_deg,
        exp_data["IFL"].to_numpy(),
        color=default_colors[1],
        label="CL interference, experiment, Deybach et al. (2026)"
    )

    ax.set_xlabel("Angular position of the interfering rotor [deg]")
    ax.set_ylabel("Interference factor")

    ax.set_ylim(0.5, 2.5)
    ax.set_xlim(-180, 180)

    ax.legend()
    plt.show()
