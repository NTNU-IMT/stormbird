'''
This script runs simulations for multiple angles of attack and compares the results to experimental 
data.

The case is a single element rectangular wing sail.

The simulation is executed in three different modes for comparison:
    1. Raw simulation - meaning no smoothing or prescribed circulation is applied
    2. Prescribed circulation - the circulation is prescribed to follow an elliptic distribution
    3. Gaussian smoothing - the circulation is smoothed using a Gaussian kernel
'''

import numpy as np
import matplotlib.pyplot as plt

import json

from simulation import SimulationCase, SimulationMode
from section_model import optimized_model

from enum import Enum

class TestCases(Enum):
    RAW_SIMULATION = 0
    PRESCRIBED_CIRCULATION = 1
    INITIALIZED_SIMULATION = 2

    def to_string(self):
        return self.name.replace("_", " ").lower()

if __name__ == "__main__":
    comparison_data = json.load(open("graf_2014_data.json", "r"))

    theoretical_aspect_ratio = 2 * 4.5

    angles_of_attack = np.arange(0.0, 16.0, 0.5)
    n_angles = len(angles_of_attack)

    section_model = optimized_model()

    cl_2d = np.zeros(n_angles)
    cd_2d = np.zeros(n_angles)

    for i in range(n_angles):
        cd_2d[i] = section_model.drag_coefficient(np.radians(angles_of_attack[i]))
        cl_2d[i] = section_model.lift_coefficient(np.radians(angles_of_attack[i]))

    cases = [TestCases.RAW_SIMULATION, TestCases.PRESCRIBED_CIRCULATION, TestCases.INITIALIZED_SIMULATION]

    w_plot = 18
    h_plot = w_plot / 2.35
    fig = plt.figure(figsize=(w_plot, h_plot))

    ax1 = fig.add_subplot(121)
    ax2 = fig.add_subplot(122)
    
    for case in cases:
        print()
        print(case.to_string())

        match case:
            case TestCases.RAW_SIMULATION:
                prescribed_circulation = False
                prescribed_initialization = False
            case TestCases.PRESCRIBED_CIRCULATION:
                prescribed_circulation = True
                prescribed_initialization = False
            case TestCases.INITIALIZED_SIMULATION:
                prescribed_circulation = False
                prescribed_initialization = True
            case _:
                raise ValueError("Invalid case")

        cl = np.zeros(n_angles)
        cd = np.zeros(n_angles)

        for angle_index in range(n_angles):
            print("Testing angle of attack: ", angles_of_attack[angle_index], " degrees")

            sim_case = SimulationCase(
                angle_of_attack = angles_of_attack[angle_index],
                simulation_mode = SimulationMode.STATIC,
                prescribed_circulation = prescribed_circulation,
                prescribed_initialization = prescribed_initialization,
                z_symmetry=True
            )

            result_history = sim_case.run()

            print("Number of iterations", result_history[-1].iterations)

            force = result_history[-1].integrated_forces[0].total

            cd[angle_index] = force.x / sim_case.force_factor
            cl[angle_index] = force.y / sim_case.force_factor

        cl_theory = cl_2d / (1 + 2/theoretical_aspect_ratio)
        cd_theory = cd_2d + cl_theory**2 / (np.pi * theoretical_aspect_ratio)

        ax1.plot(angles_of_attack, cl, label='Stormbird lifting line, ' + case.to_string())
        ax2.plot(angles_of_attack, cd, label='Stormbird lifting line, ' + case.to_string())
    
    # --------------- Comparison data ------------------------
    ax1.plot(
        comparison_data["experimental"]["angles_of_attack"], 
        comparison_data["experimental"]["lift_coefficients"], 
        "-o",
        label="Graf et al. (2014), experimental"
    )

    #ax1.plot(angles_of_attack, cl_theory, label='Section model + elliptic wing theory')
    
    ax2.plot(
        comparison_data["experimental"]["angles_of_attack"], 
        comparison_data["experimental"]["drag_coefficients"], 
        "-o",
        label="Graf et al. (2014), experimental"
    )

    #ax2.plot(angles_of_attack, cd_theory, label='Elliptic wing theory')

    ax1.set_ylim(0, 1.2)
    ax2.set_ylim(0, 0.25)
    
    ax1.set_xlabel("Angle of attack [deg]")
    ax1.set_ylabel("Lift coefficient")

    ax2.set_xlabel("Angle of attack [deg]")
    ax2.set_ylabel("Drag coefficient")


    ax2.legend()

    plt.show()