import matplotlib.pyplot as plt

import json

import numpy as np

from rotor_simulation import RotorSimulationCase, SimulationMode
from tillig_model import tillig_drag_coefficient, tillig_lift_coefficient

import argparse

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Run multiple spin ratios")
    parser.add_argument("--dynamic", action="store_true", help="Dynamic simulation")
    parser.add_argument("--smoothing-length", type=float, default=None, help="Smoothing length")

    args = parser.parse_args()

    simulation_mode = SimulationMode.DYNAMIC if args.dynamic else SimulationMode.STATIC

    spin_ratios = np.arange(0.0, 5.1, 0.25)
    n_spin_ratios = len(spin_ratios)

    cl = np.zeros(n_spin_ratios)
    cd = np.zeros(n_spin_ratios)

    with open("ostman_cfd_comparison_data.json", "r") as file:
        cfd_data = json.load(file)

    for i in range(n_spin_ratios):
        print("Testing spin ratio: ", spin_ratios[i])

        sim_case = RotorSimulationCase(
            spin_ratio=spin_ratios[i],
            smoothing_length=args.smoothing_length,
            simulation_mode=simulation_mode,
        )

        result_history = sim_case.run()

        force = result_history[-1].integrated_forces[0].total

        cd[i] = force.x / sim_case.force_factor
        cl[i] = force.y / sim_case.force_factor

    n_spin_ratios_tillig = 100
    
    spin_ratios_tillig = np.linspace(0.0, np.max(spin_ratios), n_spin_ratios_tillig)
    cl_tillig = np.zeros(n_spin_ratios_tillig)
    cd_tillig = np.zeros(n_spin_ratios_tillig)

    for i in range(n_spin_ratios_tillig):
        cl_tillig[i] = tillig_lift_coefficient(spin_ratios_tillig[i])
        cd_tillig[i] = tillig_drag_coefficient(spin_ratios_tillig[i])

    w_plot = 16
    fig = plt.figure(figsize=(w_plot, w_plot / 2.35))

    ax1 = fig.add_subplot(121)
    ax2 = fig.add_subplot(122)

    plt.sca(ax1)
    plt.plot(spin_ratios, cd)
    plt.plot(spin_ratios_tillig, cd_tillig)
    plt.plot(cfd_data["spin_ratios"], cfd_data["cd"], "o", label="CFD data")

    plt.xlabel("Spin ratio")
    plt.ylabel("Drag coefficient")

    plt.ylim(0, 5.0)
    plt.xlim(0, 5.1)

    plt.sca(ax2)
    plt.plot(spin_ratios, cl, label="Lifting line simulation")
    plt.plot(spin_ratios_tillig, cl_tillig, label="Empirical model from Tillig et al. (2020)")
    plt.plot(cfd_data["spin_ratios"], cfd_data["cl"], "o", label="CFD data from Östman et al. (2023)")

    plt.ylim(0, 14.0)
    plt.xlim(0, 5.1)

    plt.xlabel("Spin ratio")
    plt.ylabel("Lift coefficient")

    plt.legend(loc="upper left")

    plt.show()