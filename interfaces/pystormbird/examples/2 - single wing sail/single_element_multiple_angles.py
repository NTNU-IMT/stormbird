'''
This script runs simulations for multiple angles of attack and compares the results to experimental 
data.
'''

import sys

import numpy as np
import matplotlib.pyplot as plt

import json

import time as time_func

from simulation import SimulationCase, SolverType

section_model_path = '../1 - section models'

if section_model_path not in sys.path:
    sys.path.append(section_model_path)

import naca_0012_Graf2014

if __name__ == "__main__":
    comparison_data = json.load(open("../comparison_data/graf_2014_data.json", "r"))

    theoretical_aspect_ratio = 2 * 4.5

    angles_of_attack = np.arange(0.0, 20.0, 0.5)
    n_angles = len(angles_of_attack)

    foil_tuner = naca_0012_Graf2014.get_tuned_foil_tuner()
    foil_model = foil_tuner.get_foil_model()
    section_model = foil_tuner.get_section_model_setup()

    cl_2d = np.zeros(n_angles)
    cd_2d = np.zeros(n_angles)

    for i in range(n_angles):
        cd_2d[i] = foil_model.drag_coefficient(np.radians(angles_of_attack[i]))
        cl_2d[i] = foil_model.lift_coefficient(np.radians(angles_of_attack[i]))

    dynamic = [False, True, False, True]
    solver_types = [SolverType.Linearized, SolverType.Linearized, SolverType.SimpleIterative, SolverType.SimpleIterative]

    w_plot = 18
    h_plot = w_plot / 2.35
    fig = plt.figure(figsize=(w_plot, h_plot))

    ax1 = fig.add_subplot(121)
    ax2 = fig.add_subplot(122)

    for dyn, solver in zip(dynamic, solver_types):
        label = "Dynamic" if dyn else "Quasi-steady"
        label += " - " + solver.name

        start_time = time_func.time()
        print()
        print(label)

        cl = np.zeros(n_angles)
        cd = np.zeros(n_angles)

        for angle_index in range(n_angles):
            print("Testing angle of attack: ", angles_of_attack[angle_index], " degrees")

            sim_case = SimulationCase(
                angle_of_attack = angles_of_attack[angle_index],
                section_model = section_model,
                dynamic = dyn,
                solver_type = solver,
                z_symmetry = True
            )

            result_history = sim_case.run()

            print("Number of iterations", result_history[-1].iterations)

            force = result_history[-1].integrated_forces[0].total

            cd[angle_index] = force.x / sim_case.force_factor
            cl[angle_index] = force.y / sim_case.force_factor

        print("Elapsed time: ", time_func.time() - start_time)

        cl_theory = cl_2d / (1 + 2/theoretical_aspect_ratio)
        cd_theory = cd_2d + cl_theory**2 / (np.pi * theoretical_aspect_ratio)

        

        ax1.plot(angles_of_attack, cl, label='Lifting line, ' + label)
        ax2.plot(angles_of_attack, cd, label='Lifting line, ' + label)


    # --------------- Comparison data ------------------------
    ax1.plot(
        comparison_data["experimental"]["angles_of_attack"], 
        comparison_data["experimental"]["lift_coefficients"], 
        "-o",
        label="Graf et al. (2014), experimental"
    )

    ax1.plot(angles_of_attack, cl_2d, label='Section model', color='grey', linestyle='--')
    
    ax2.plot(
        comparison_data["experimental"]["angles_of_attack"], 
        comparison_data["experimental"]["drag_coefficients"], 
        "-o",
        label="Graf et al. (2014), experimental"
    )

    ax2.plot(angles_of_attack, cd_2d, label='Section model', color='grey', linestyle='--')

    #ax1.set_ylim(0, 1.2)
    ax2.set_ylim(0, 0.25)
    
    ax1.set_xlabel("Angle of attack [deg]")
    ax1.set_ylabel("Lift coefficient")

    ax2.set_xlabel("Angle of attack [deg]")
    ax2.set_ylabel("Drag coefficient")


    ax2.legend()

    plt.show()