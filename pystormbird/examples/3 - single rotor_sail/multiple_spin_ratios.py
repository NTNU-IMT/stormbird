import matplotlib.pyplot as plt

import json

import numpy as np

from rotor_simulation import RotorSimulationCase, SimulationMode, TestCase
from tillig_model import tillig_drag_coefficient, tillig_lift_coefficient

import argparse

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Run multiple spin ratios")
    parser.add_argument("--smoothing-length", type=float, default=None, help="Smoothing length")

    args = parser.parse_args()

    w_plot = 16
    fig = plt.figure(figsize=(w_plot, w_plot / 2.35))

    ax1 = fig.add_subplot(121)
    ax2 = fig.add_subplot(122)

    simulation_mode = SimulationMode.STATIC

    cases = [
        TestCase.RAW_SIMULATION,
        TestCase.LIMIT_ON_INDUCED_VELOCITY,
        TestCase.FIXED_VELOCITY_MAGNITUDE,
        TestCase.VIRTUAL_END_DISK
    ]

    for case in cases:
        spin_ratios = np.arange(0.0, 5.1, 0.25)
        n_spin_ratios = len(spin_ratios)

        cl = np.zeros(n_spin_ratios)
        cd = np.zeros(n_spin_ratios)

        for i in range(n_spin_ratios):
            print("Testing spin ratio: ", spin_ratios[i])

            sim_case = RotorSimulationCase(
                spin_ratio=spin_ratios[i],
                simulation_mode=simulation_mode,
                test_case=case
            )

            result_history = sim_case.run()

            force = result_history[-1].integrated_forces[0].total

            cd[i] = force.x / sim_case.force_factor
            cl[i] = force.y / sim_case.force_factor

        ax1.plot(spin_ratios, cd)
        ax2.plot(spin_ratios, cl, label=case.to_string())

    n_spin_ratios_tillig = 100
    spin_ratios_tillig = np.linspace(0.0, np.max(spin_ratios), n_spin_ratios_tillig)
    cl_tillig = np.zeros(n_spin_ratios_tillig)
    cd_tillig = np.zeros(n_spin_ratios_tillig)

    for i in range(n_spin_ratios_tillig):
        cl_tillig[i] = tillig_lift_coefficient(spin_ratios_tillig[i])
        cd_tillig[i] = tillig_drag_coefficient(spin_ratios_tillig[i])

    ax1.plot(spin_ratios_tillig, cd_tillig)
    ax2.plot(spin_ratios_tillig, cl_tillig, label="Empirical model from Tillig et al. (2020)")

    
    with open("../comparison_data/ostman_cfd_comparison_data.json", "r") as file:
        cfd_data = json.load(file)

    ax1.plot(cfd_data["spin_ratios"], cfd_data["cd"], "o", label="CFD data")
    ax2.plot(cfd_data["spin_ratios"], cfd_data["cl"], "o", label="CFD data from Östman et al. (2023)")

    ax1.set_xlabel("Spin ratio")
    ax1.set_ylabel("Drag coefficient")

    ax1.set_ylim(0, 5.0)
    ax1.set_xlim(0, 5.1)

    ax2.set_ylim(0, 14.0)
    ax2.set_xlim(0, 5.1)

    ax2.set_xlabel("Spin ratio")
    ax2.set_ylabel("Lift coefficient")

    ax2.legend(loc="upper left")

    plt.show()