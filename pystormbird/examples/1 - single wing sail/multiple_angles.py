import numpy as np
import matplotlib.pyplot as plt

import json

import argparse

from simulation import SimulationCase, SimulationMode

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Run a single case")
    parser.add_argument("--dynamic", action="store_true", help="Use dynamic model")
    parser.add_argument("--smoothing-length", type=float, default=None, help="Smoothing length")

    args = parser.parse_args()

    comparison_data = json.load(open("graf_2014_data.json", "r"))

    theoretical_aspect_ratio = 2 * 4.5

    simulation_mode = SimulationMode.DYNAMIC if args.dynamic else SimulationMode.STATIC

    angles_of_attack = np.arange(0.0, 16.0, 0.5)
    n_angles = len(angles_of_attack)

    cl = np.zeros(n_angles)
    cd = np.zeros(n_angles)

    for i in range(n_angles):
        print("Testing angle of attack: ", angles_of_attack[i], " degrees")

        sim_case = SimulationCase(
            angle_of_attack = angles_of_attack[i],
            simulation_mode = simulation_mode,
            smoothing_length = args.smoothing_length,
            z_symmetry=True
        )

        result_history = sim_case.run()

        print("Number of iterations", result_history[-1].iterations)

        force = result_history[-1].integrated_forces[0].total

        cd[i] = force.x / sim_case.force_factor
        cl[i] = force.y / sim_case.force_factor

    cl_theory = 2 * np.pi * np.radians(angles_of_attack) / (1 + 2/theoretical_aspect_ratio)
    cd_theory = cd[0] + cl_theory**2 / (np.pi * theoretical_aspect_ratio)

    w_plot = 18
    h_plot = w_plot / 2.35
    fig = plt.figure(figsize=(w_plot, h_plot))

    ax1 = fig.add_subplot(121)
    ax2 = fig.add_subplot(122)

    plt.sca(ax1)
    plt.plot(angles_of_attack, cl, label='Stormbird lifting line')
    plt.plot(
        comparison_data["experimental"]["angles_of_attack"], 
        comparison_data["experimental"]["lift_coefficients"], 
        "-o",
        label="Graf et al. (2014), experimental"
    )

    plt.plot(angles_of_attack, cl_theory, label='Elliptic wing theory')

    plt.ylim(0, 1.2)

    

    plt.sca(ax2)
    plt.plot(angles_of_attack, cd, label='Stormbird lifting line')
    plt.plot(
        comparison_data["experimental"]["angles_of_attack"], 
        comparison_data["experimental"]["drag_coefficients"], 
        "-o",
        label="Graf et al. (2014), experimental"
    )

    plt.plot(angles_of_attack, cd_theory, label='Elliptic wing theory')

    plt.ylim(0, 0.25)

    plt.legend()

    plt.show()