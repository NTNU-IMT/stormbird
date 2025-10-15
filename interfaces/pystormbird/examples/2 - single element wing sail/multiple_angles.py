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
    comparison_data = json.load(open("../comparison_data/graf_2014_data.json", "r"))

    angles_of_attack_deg = np.arange(0.0, 20.5, 0.5)
    n_angles = len(angles_of_attack_deg)

    dynamic = [False, False, False]
    solver_types = [SolverType.Linearized, SolverType.SimpleIterative, 
                    SolverType.SimpleIterative, SolverType.SimpleIterative]
    smoothing_length = [0.0, 0.0, 0.1, 0.1]

    w_plot = 18
    h_plot = w_plot / 2.35
    fig = plt.figure(figsize=(w_plot, h_plot))

    ax1 = fig.add_subplot(121)
    ax2 = fig.add_subplot(122)

    for dyn, solver, smoothing in zip(dynamic, solver_types, smoothing_length):
        label = "Dynamic" if dyn else "Quasi-steady"
        label += " - " + solver.name

        if smoothing > 0.0:
            label += f", smoothing {smoothing:.1f}"

        print()
        print(label)

        cl = np.zeros(n_angles)
        cd = np.zeros(n_angles)

        for angle_index in range(n_angles):
            print("Testing angle of attack: ", angles_of_attack_deg[angle_index], " degrees")

            res = simulate_single_case(
                angle_of_attack_deg = angles_of_attack_deg[angle_index],
                solver_type = solver,
                dynamic = dyn,
                smoothing_length = smoothing
            )

            cl[angle_index] = res['cl']
            cd[angle_index] = res['cd']
    
        ax1.plot(angles_of_attack_deg, cl, label='Lifting line, ' + label)
        ax2.plot(angles_of_attack_deg, cd, label='Lifting line, ' + label)


    # --------------- Comparison data ------------------------
    ax1.plot(
        comparison_data["experimental"]["angles_of_attack"], 
        comparison_data["experimental"]["lift_coefficients"], 
        "-o",
        label="Graf et al. (2014), experimental"
    )
    
    ax2.plot(
        comparison_data["experimental"]["angles_of_attack"], 
        comparison_data["experimental"]["drag_coefficients"], 
        "-o",
        label="Graf et al. (2014), experimental"
    )

    #ax1.set_ylim(0, 1.2)
    #ax2.set_ylim(0, 0.25)
    
    ax1.set_xlabel("Angle of attack [deg]")
    ax1.set_ylabel("Lift coefficient")

    ax2.set_xlabel("Angle of attack [deg]")
    ax2.set_ylabel("Drag coefficient")


    ax2.legend()

    plt.show()