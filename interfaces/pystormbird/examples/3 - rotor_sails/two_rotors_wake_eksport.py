"""
This example compares the output from a lifting line model with two rotor sails, against the 

"""

import argparse

import numpy as np
import matplotlib.pyplot as plt
from stormbird_setup.spatial_vector import SpatialVector
from stormbird_setup.simplified_setup.single_wing_simulation import SolverType

import pandas as pd

from setup import simulate_single_case

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Run a case with two rotors")
    parser.add_argument("--dynamic-shape", action="store_true", help="Turns on dynamic shape for the dynamic wake")

    args = parser.parse_args()
    
    default_colors = plt.rcParams['axes.prop_cycle'].by_key()['color']

    exp_data = pd.read_csv("data/deybach_2026_interference_factors.csv", delimiter=";")
    
    spin_ratio = 2.5

    diameter = 5.0
    height = 35.0
    foundation_height = 1.875

    spacing = 4.0 * diameter

    rotation_deg = 10.0

    wind_direction_deg = 0.0

    w_plot = 16
    h_plot = w_plot / 1.85
    fig = plt.figure(figsize=(w_plot, h_plot))

    ax = fig.add_subplot(111)

    test_index = 2

    solver = SolverType.SimpleIterative
    max_induced_velocity_ratio = 0.0
    smoothing_length = 0.0

    interfering_rotor_location = SpatialVector(x=-spacing).rotate_around_axis(
        np.radians(rotation_deg),
        axis=SpatialVector(z=1.0)
    )

    res = simulate_single_case(
        diameter = diameter,
        height = height,
        foundation_height=foundation_height,
        rotor_x_locations = [0.0, interfering_rotor_location.x],
        rotor_y_locations = [0.0, interfering_rotor_location.y],
        spin_ratio = spin_ratio,
        solver_type = solver,
        max_induced_velocity_ratio = max_induced_velocity_ratio,
        smoothing_length = smoothing_length,
        wind_direction_deg=wind_direction_deg,
        dynamic = True,
        dynamic_shape = args.dynamic_shape,
        write_wake = True
    )
