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

The function simulate_interference_factors is also used by the actuator line version of this
example, in interfaces/pystormflow/examples/rotor_sails/deybach_wind_tunnel, to compare against the
lifting line results.
"""

from pathlib import Path

import numpy as np
import matplotlib.pyplot as plt
from stormbird_setup.spatial_vector import SpatialVector

import polars as pl

from setup import simulate_single_case, SolverType

EXPERIMENTAL_DATA_PATH = Path(__file__).parent / "data" / "deybach_2026_interference_factors.csv"

SPIN_RATIO = 2.5

DIAMETER = 5.0
HEIGHT = 30.0
FOUNDATION_HEIGHT = 1.875

SPACING = 4.0 * DIAMETER

WIND_DIRECTION_DEG = 0.0

# The variants of the simulation that are compared. Each variant gets its own line style in the plot
QUASI_STEADY_VARIANT = {
    "label": "Quasi-steady",
    "dynamic": False,
    "line_style": "-",
}

VARIANTS = [
    QUASI_STEADY_VARIANT,
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

def read_experimental_data() -> pl.DataFrame:
    return pl.read_csv(EXPERIMENTAL_DATA_PATH, separator=";")

def experimental_angular_positions_deg() -> np.ndarray:
    return read_experimental_data()["Angular position of the interfering rotor [deg]"].to_numpy()

def interfering_rotor_location(angular_position_deg: float) -> SpatialVector:
    '''
    Location of the interfering rotor. The other rotor is at the origin, and an angular position
    of 0 degrees means that the interfering rotor is directly upstream.
    '''
    return SpatialVector(x=-SPACING).rotate_around_axis(
        np.radians(angular_position_deg),
        axis=SpatialVector(z=1.0)
    )

def simulate_interference_factors(
    variant: dict,
    angular_positions_deg: np.ndarray
) -> tuple[np.ndarray, np.ndarray]:
    '''
    Computes the drag and lift interference factors on the rotor at the origin, for each angular
    position of the interfering rotor. Returns the drag and lift interference factors.
    '''
    simulation_settings = dict(
        diameter = DIAMETER,
        height = HEIGHT,
        foundation_height = FOUNDATION_HEIGHT,
        spin_ratio = SPIN_RATIO,
        solver_type = SolverType.Iterative,
        wind_direction_deg = WIND_DIRECTION_DEG,
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

    n_rotations = len(angular_positions_deg)

    cd = np.zeros(n_rotations)
    cl = np.zeros(n_rotations)

    for dir_index in range(n_rotations):
        print("Testing rotation: ", angular_positions_deg[dir_index])

        location = interfering_rotor_location(angular_positions_deg[dir_index])

        res = simulate_single_case(
            rotor_x_locations = [0.0, location.x],
            rotor_y_locations = [0.0, location.y],
            **simulation_settings
        )

        cd[dir_index] = res[0]['cx']
        cl[dir_index] = res[0]['cy']

    return cd / cd_single, cl / cl_single

if __name__ == "__main__":
    default_colors = plt.rcParams['axes.prop_cycle'].by_key()['color']

    exp_data = read_experimental_data()

    rotations_deg = experimental_angular_positions_deg()

    w_plot = 16
    h_plot = w_plot / 1.85
    fig = plt.figure(figsize=(w_plot, h_plot))

    ax = fig.add_subplot(111)

    for variant in VARIANTS:
        print()
        print(variant["label"])

        cd_interference, cl_interference = simulate_interference_factors(variant, rotations_deg)

        ax.plot(
            rotations_deg, cd_interference,
            variant["line_style"],
            color=default_colors[0],
            label="CD interference, " + variant["label"]
        )

        ax.plot(
            rotations_deg, cl_interference,
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
