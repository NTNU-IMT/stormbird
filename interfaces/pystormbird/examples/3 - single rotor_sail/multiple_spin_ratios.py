'''
This script runs simulations for a single wing sail at multiple angles of attack and compares the 
results to experimental data. The simulations are executed with various settings for comparison.
'''

import numpy as np
import matplotlib.pyplot as plt

import json

from stormbird_setup.simplified_setup.single_wing_simulation import SolverType

from single_case import simulate_single_case

if __name__ == "__main__":
    comparison_data = json.load(open("../comparison_data/ostman_cfd_comparison_data.json", "r"))
    
    spin_ratio = np.arange(0.0, 6.0, 0.25)
    n_spin_ratios = len(spin_ratio)

    max_induced_velocity_ratios = [0.0, 1.0, 1.0]
    solver_types = [SolverType.Linearized, SolverType.Linearized, SolverType.SimpleIterative]

    w_plot = 18
    h_plot = w_plot / 2.35
    fig = plt.figure(figsize=(w_plot, h_plot))

    ax1 = fig.add_subplot(121)
    ax2 = fig.add_subplot(122)

    for index, solver in enumerate(solver_types):
        label = solver.name

        if max_induced_velocity_ratios[index] == 0.0:
            label += ", no induced velocity limit"
        else:
            label += f", max induced velocity ratio {max_induced_velocity_ratios[index]:.1f}"

        print()
        print(label)

        cl = np.zeros(n_spin_ratios)
        cd = np.zeros(n_spin_ratios)

        for spin_index in range(n_spin_ratios):
            print("Testing spin ratio: ", spin_ratio[spin_index])

            res = simulate_single_case(
                spin_ratio = spin_ratio[spin_index],
                solver_type = solver,
                max_induced_velocity_ratio = max_induced_velocity_ratios[index]
            )

            cl[spin_index] = res['cl']
            cd[spin_index] = res['cd']

        ax1.plot(spin_ratio, cl, label='Lifting line, ' + label)
        ax2.plot(spin_ratio, cd, label='Lifting line, ' + label)


    # --------------- Comparison data ------------------------
    spin_ratios_tillig = np.linspace(0.0, 6.0, 100)
    from tillig_model import tillig_lift_coefficient, tillig_drag_coefficient
    cl_tillig = np.array([tillig_lift_coefficient(sr) for sr in spin_ratios_tillig])
    cd_tillig = np.array([tillig_drag_coefficient(sr) for sr in spin_ratios_tillig])

    ax1.plot(spin_ratios_tillig, cl_tillig, label="Empirical model, Tillig et al. (2020)")
    ax2.plot(spin_ratios_tillig, cd_tillig, label="Empirical model, Tillig et al. (2020)")

    ax1.plot(
        comparison_data["spin_ratios"], 
        comparison_data["cl"], 
        "-o",
        label="CFD, Ostman et al. (2022)"
    )
    
    ax2.plot(
        comparison_data["spin_ratios"], 
        comparison_data["cd"], 
        "-o",
        label="CFD, Ostman et al. (2022)"
    )

    #ax1.set_ylim(0, 1.2)
    #ax2.set_ylim(0, 0.25)

    ax1.set_xlabel("Spin ratio")
    ax1.set_ylabel("Lift coefficient")

    ax2.set_xlabel("Spin ratio")
    ax2.set_ylabel("Drag coefficient")


    ax2.legend()

    plt.show()