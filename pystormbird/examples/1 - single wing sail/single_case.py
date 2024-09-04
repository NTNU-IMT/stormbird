import numpy as np
import matplotlib.pyplot as plt

import argparse

from simulation import SimulationCase, SimulationMode

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Run a single case")
    parser.add_argument("--angle-of-attack", type=float, default = 0.0, help="Angle of attack in degrees")
    parser.add_argument("--smoothing-length", type=float, default = None, help="Use smoothing length")
    parser.add_argument("--dynamic", action="store_true", help="Use dynamic model")
    
    parser.add_argument("--write-wake-files", action="store_true", help="Write wake files")


    args = parser.parse_args()

    simulation_mode = SimulationMode.DYNAMIC if args.dynamic else SimulationMode.STATIC

    sim_case = SimulationCase(
        angle_of_attack = args.angle_of_attack,
        smoothing_length = args.smoothing_length,
        simulation_mode = simulation_mode,
        write_wake_files=args.write_wake_files
    )

    result_history = sim_case.run()

    cd = np.zeros(len(result_history))
    cl = np.zeros(len(result_history))

    for i in range(len(result_history)):
        force = result_history[i].integrated_forces[0].total

        cd[i] = force.x / sim_case.force_factor
        cl[i] = force.y / sim_case.force_factor

    print('Last lift coefficient:', cl[-1])
    print('Last drag coefficient:', cd[-1])

    circulation_strength = np.array(result_history[-1].force_input.circulation_strength)
    angles_of_attack     = np.array(result_history[-1].force_input.angles_of_attack)

    w_plot = 16
    fig = plt.figure(figsize=(w_plot, w_plot/3.0))
    ax1 = fig.add_subplot(131)
    ax2 = fig.add_subplot(132)
    ax3 = fig.add_subplot(133)

    plt.sca(ax1)
    if len(cl) > 1:
        plt.plot(cl)
    else:
        plt.plot([0, 1], [cl[0], cl[0]])

    plt.sca(ax2)
    plt.plot(-circulation_strength)

    plt.sca(ax3)
    plt.plot(np.degrees(angles_of_attack))

    plt.show()
