import numpy as np
import matplotlib.pyplot as plt

import json

import argparse

from simulation import SimulationCase, SimulationMode

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Run a single case")
    parser.add_argument("--dynamic", action="store_true", help="Use dynamic model")
    parser.add_argument("--smoothing-length", type=float, default=None, help="Smoothing length")
    parser.add_argument("--use-start-angle", action="store_true", help="Use start angle of attack")

    args = parser.parse_args()

    comparison_data = json.load(open("graf_2014_data.json", "r"))

    simulation_mode = SimulationMode.DYNAMIC if args.dynamic else SimulationMode.STATIC

    angles_of_attack = np.arange(0.0, 16.0, 0.5)
    n_angles = len(angles_of_attack)

    cl = np.zeros(n_angles)
    cd = np.zeros(n_angles)

    for i in range(n_angles):
        print("Testing angle of attack: ", angles_of_attack[i], " degrees")

        if angles_of_attack[i] > 10 and args.use_start_angle:
            start_angle = 10.0
        else:
            start_angle = None

        sim_case = SimulationCase(
            angle_of_attack = angles_of_attack[i],
            start_angle_of_attack = start_angle,
            simulation_mode = simulation_mode,
            smoothing_length = args.smoothing_length,
            z_symmetry=True
        )

        result_history = sim_case.run()

        force = result_history[-1].integrated_forces[0].total

        cd[i] = force.x / sim_case.force_factor
        cl[i] = force.y / sim_case.force_factor

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

    plt.ylim(0, 1.2)

    plt.legend()

    plt.sca(ax2)
    plt.plot(angles_of_attack, cd)
    plt.plot(
        comparison_data["experimental"]["angles_of_attack"], 
        comparison_data["experimental"]["drag_coefficients"], 
        "-o",
        label="Graf et al. (2014), experimental"
    )

    plt.ylim(0, 0.25)

    plt.show()