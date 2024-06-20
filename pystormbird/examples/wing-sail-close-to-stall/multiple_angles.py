import numpy as np
import matplotlib.pyplot as plt

import argparse

from simulation import run_simulation, SimulationCase

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Run a single case")
    parser.add_argument("--dynamic", action="store_true", help="Use dynamic model")
    parser.add_argument("--smoothing", action="store_true", help="Use smoothing")
    parser.add_argument("--use-start-angle", action="store_true", help="Use start angle of attack")

    args = parser.parse_args()

    simulation_type = "dynamic" if args.dynamic else "static"

    angles_of_attack = np.arange(0, 35.0, 1)
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
            simulation_type = simulation_type,
            smoothing = args.smoothing
        )

        result_history = run_simulation(sim_case)

        force = result_history[-1].integrated_forces[0]

        cd[i] = force.x / sim_case.force_factor
        cl[i] = force.y / sim_case.force_factor

    fig = plt.figure()
    ax1 = fig.add_subplot(121)
    ax2 = fig.add_subplot(122)

    plt.sca(ax1)
    plt.plot(angles_of_attack, cl)

    plt.ylim(0, 2.5)

    plt.sca(ax2)
    plt.plot(angles_of_attack, cd)

    plt.ylim(0, 0.5)

    plt.show()