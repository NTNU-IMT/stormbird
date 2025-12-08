'''
This script runs simulations for a single rotors sail at multiple spin ratios and compares the 
results to CFD data. The simulations are executed with various settings, for a test to see what 
settings works best.
'''

import numpy as np
import matplotlib.pyplot as plt

import json

from setup import simulate_single_case, TEST_SETTINGS

if __name__ == "__main__":
    comparison_data = json.load(open("../comparison_data/ostman_cfd_comparison_data.json", "r"))
    
    spin_ratio = np.arange(0.0, 5.5, 0.25)
    n_spin_ratios = len(spin_ratio)

    w_plot = 18
    h_plot = w_plot / 2.35
    fig = plt.figure(figsize=(w_plot, h_plot))

    ax1 = fig.add_subplot(121)
    ax2 = fig.add_subplot(122)

    for index in range(len(TEST_SETTINGS["solver_types"])):
        solver = TEST_SETTINGS["solver_types"][index]
        max_induced_velocity_ratio = TEST_SETTINGS["max_induced_velocity_ratios"][index]
        smoothing_length = TEST_SETTINGS["smoothing_lengths"][index]
        
        label = solver.name

        if max_induced_velocity_ratio > 0.0:
            label += f", max u_i ratio {max_induced_velocity_ratio:.1f}"
        if smoothing_length > 0.0:
            label += f", smoothing length {smoothing_length:.3f}"

        print()
        print(label)

        cl = np.zeros(n_spin_ratios)
        cd = np.zeros(n_spin_ratios)

        for spin_index in range(n_spin_ratios):
            print("Testing spin ratio: ", spin_ratio[spin_index])

            res = simulate_single_case(
                rotor_x_locations = [0.0],
                rotor_y_locations = [0.0],
                spin_ratio = spin_ratio[spin_index],
                solver_type = solver,
                max_induced_velocity_ratio = max_induced_velocity_ratio,
                smoothing_length = smoothing_length
            )[0]

            cl[spin_index] = res['cy']
            cd[spin_index] = res['cx']

        ax1.plot(spin_ratio, cl, label='Lifting line, ' + label)
        ax2.plot(spin_ratio, cd, label='Lifting line, ' + label)


    # --------------- Comparison data ------------------------
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
    
    ax1.set_xlim(0.0, 5.25)
    ax1.set_ylim(0.0, 12.0)
    
    ax2.set_xlim(0.0, 5.25)
    ax2.set_ylim(0.0, 5.0)
        
    ax1.set_xlabel("Spin ratio")
    ax1.set_ylabel("Lift coefficient")

    ax2.set_xlabel("Spin ratio")
    ax2.set_ylabel("Drag coefficient")

    ax1.legend(loc=4)

    plt.show()
