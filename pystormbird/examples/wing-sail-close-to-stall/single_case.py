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
    parser.add_argument("--write-wake-files", action="store_true", help="Write wake files")


    args = parser.parse_args()

    simulation_type = "dynamic" if args.dynamic else "static"

    sim_case = SimulationCase(
        angle_of_attack = args.angle_of_attack,
        start_angle_of_attack = args.start_angle_of_attack,
        simulation_type = simulation_type,
        smoothing = args.smoothing,
        write_wake_files=args.write_wake_files
    )

    result_history = run_simulation(sim_case)

    cd = np.zeros(len(result_history))
    cl = np.zeros(len(result_history))

    for i in range(len(result_history)):
        force = result_history[i].integrated_forces[0]

        cd[i] = force.x
        cl[i] = force.y

    circulation_strength = np.array(result_history[-1].circulation_strength)
    angle_of_attack = np.array(result_history[-1].angle_of_attack)

    w_plot = 16
    fig = plt.figure(figsize=(w_plot, w_plot/3.0))
    ax1 = fig.add_subplot(131)
    ax2 = fig.add_subplot(132)
    ax3 = fig.add_subplot(133)

    plt.sca(ax1)
    plt.plot(cl)

    plt.sca(ax2)
    plt.plot(-circulation_strength)

    plt.sca(ax3)
    plt.plot(angle_of_attack)

    plt.show()
