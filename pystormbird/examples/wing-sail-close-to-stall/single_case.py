import numpy as np
import matplotlib.pyplot as plt

import argparse

from simulation import run_simulation, SimulationCase

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Run a single case")
    parser.add_argument("--angle-of-attack", type=float, default = 0.0, help="Angle of attack in degrees")
    parser.add_argument("--start-angle-of-attack", type=float, default = None, help="Start angle of attack in degrees")
    parser.add_argument("--dynamic", action="store_true", help="Use dynamic model")
    parser.add_argument("--smoothing", action="store_true", help="Use smoothing")


    args = parser.parse_args()

    simulation_type = "dynamic" if args.dynamic else "static"

    sim_case = SimulationCase(
        angle_of_attack = args.angle_of_attack,
        start_angle_of_attack = args.start_angle_of_attack,
        simulation_type = simulation_type,
        smoothing = args.smoothing
    )

    result_history = run_simulation(sim_case)

    cd = np.zeros(len(result_history))
    cl = np.zeros(len(result_history))

    for i in range(len(result_history)):
        force = result_history[i].integrated_forces[0]

        cd[i] = force.x
        cl[i] = force.y

    circulation_strength = np.array(result_history[-1].circulation_strength)

    fig = plt.figure()
    ax1 = fig.add_subplot(121)
    ax2 = fig.add_subplot(122)

    plt.sca(ax1)
    plt.plot(cl)

    plt.sca(ax2)
    plt.plot(-circulation_strength)

    plt.show()
