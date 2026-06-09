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
    parser.add_argument("--dynamic", action="store_true", help="Turns on dynamic wake")
    parser.add_argument("--dynamic-shape", action="store_true", help="Turns on dynamic shape for the dynamic wake")

    args = parser.parse_args()
    
    default_colors = plt.rcParams['axes.prop_cycle'].by_key()['color']

    exp_data = pd.read_csv("data/deybach_2026_interference_factors.csv", delimiter=";")
    
    spin_ratio = 2.5

    diameter = 5.0
    height = 35.0
    foundation_height = 1.875

    spacing = 4.0 * diameter

    rotations_deg = exp_data["Angular position of the interfering rotor [deg]"]
    n_rotations = len(rotations_deg)

    wind_direction_deg = 0.0

    w_plot = 16
    h_plot = w_plot / 1.85
    fig = plt.figure(figsize=(w_plot, h_plot))

    ax = fig.add_subplot(111)

    test_index = 2

    solver = SolverType.SimpleIterative
    max_induced_velocity_ratio = 0.0
    smoothing_length = 0.0

    cd1 = np.zeros(n_rotations)
    cl1 = np.zeros(n_rotations)
    
    cd2 = np.zeros(n_rotations)
    cl2 = np.zeros(n_rotations)

    cd_single = np.zeros(n_rotations)
    cl_single = np.zeros(n_rotations)

    for dir_index in range(n_rotations):
        print("Testing rotation: ", rotations_deg[dir_index])

        interfering_rotor_location = SpatialVector(x=-spacing).rotate_around_axis(
            np.radians(rotations_deg[dir_index]),
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
            dynamic = args.dynamic,
            dynamic_shape = args.dynamic_shape
        )

        res_single = simulate_single_case(
            diameter = diameter,
            height = height,
            foundation_height=foundation_height,
            rotor_x_locations = [0.0],
            rotor_y_locations = [0.0],
            spin_ratio = spin_ratio,
            solver_type = solver,
            max_induced_velocity_ratio = 2.0,
            smoothing_length = smoothing_length,
            wind_direction_deg=wind_direction_deg,
            dynamic = args.dynamic,
            dynamic_shape = args.dynamic_shape
        )

        cd1[dir_index] = res[0]['cx']
        cl1[dir_index] = res[0]['cy']
        
        cd2[dir_index] = res[1]['cx']
        cl2[dir_index] = res[1]['cy']

        cd_single[dir_index] = res_single[0]['cx']
        cl_single[dir_index] = res_single[0]['cy']
    
    ax.plot(rotations_deg, cd1 / cd_single, label="CD interference, LL", color=default_colors[0])
    ax.plot(rotations_deg, cl1 / cl_single, label="CL interference, LL", color=default_colors[1])

exp_data = pd.read_csv("data/deybach_2026_interference_factors.csv", delimiter=";")

ax.scatter(
    exp_data["Angular position of the interfering rotor [deg]"], 
    exp_data["IFD"], 
    color=default_colors[0],
    label="CD interference, exp"
)
ax.scatter(
    exp_data["Angular position of the interfering rotor [deg]"], 
    exp_data["IFL"], 
    color=default_colors[1],
    label="CL interference, exp"
)

ax.set_ylim(0.5, 2.5)
ax.set_xlim(-180, 180)

ax.legend()
plt.show()
