import matplotlib.pyplot as plt

import numpy as np

from rotor_simulation import RotorSimulationCase, SimulationMode

import argparse

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Run a single case")
    parser.add_argument("--spin-ratio", type=float, default = 2.0, help="Spin ratio")
    parser.add_argument("--dynamic", action="store_true", help="Dynamic simulation")

    args = parser.parse_args()

    simulation_mode = SimulationMode.DYNAMIC if args.dynamic else SimulationMode.STATIC

    sim_case = RotorSimulationCase(
        spin_ratio=args.spin_ratio,
        simulation_mode=simulation_mode
    )

    result_history = sim_case.run()

    cd = np.zeros(len(result_history))
    cl = np.zeros(len(result_history))

    for i in range(len(result_history)):

        force = result_history[i].integrated_forces[0].total

        cd[i] = force.x / sim_case.force_factor
        cl[i] = force.y / sim_case.force_factor

    ctrl_points = result_history[-1].ctrl_points
    circulation_strength = result_history[-1].force_input.circulation_strength

    ctrl_points_z = np.array([ctrl_point.z for ctrl_point in ctrl_points])
    
    print("Last Cl: ", cl[-1])
    print("Last Cd: ", cd[-1])

    w_plot = 16
    fig = plt.figure(figsize=(w_plot, w_plot / 2.35))
    ax1 = fig.add_subplot(121)
    ax2 = fig.add_subplot(122)

    plt.sca(ax1)
    plt.plot(cl)
    plt.plot(cd)

    plt.sca(ax2)
    plt.plot(ctrl_points_z, circulation_strength)

    plt.show()