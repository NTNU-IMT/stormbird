import sys

import numpy as np
import matplotlib.pyplot as plt

from simulation import SimulationCase, SimulationMode, TestCase

section_model_path = '../1 - section models'

if section_model_path not in sys.path:
    sys.path.append(section_model_path)

import manual_multi_element_foil

if __name__ == "__main__":
    theoretical_aspect_ratio = 2 * 4.5

    angles_of_attack = np.arange(0.0, 25.0, 0.5)
    n_angles = len(angles_of_attack)

    foil_model = manual_multi_element_foil.get_foil_model()

    foil_model.set_internal_state(np.radians(15.0))

    section_model_dict = {
        "VaryingFoil": foil_model.__dict__
    }

    cl_2d = np.zeros(n_angles)
    cd_2d = np.zeros(n_angles)

    for i in range(n_angles):
        cd_2d[i] = foil_model.drag_coefficient(np.radians(angles_of_attack[i]))
        cl_2d[i] = foil_model.lift_coefficient(np.radians(angles_of_attack[i]))

    w_plot = 18
    h_plot = w_plot / 2.35
    fig = plt.figure(figsize=(w_plot, h_plot))

    ax1 = fig.add_subplot(121)
    ax2 = fig.add_subplot(122)

    dynamic = [False, True]
    
    for dyn in dynamic:
        print()
        print(case.to_string())

        cl = np.zeros(n_angles)
        cd = np.zeros(n_angles)

        for angle_index in range(n_angles):
            print("Testing angle of attack: ", angles_of_attack[angle_index], " degrees")

            sim_case = SimulationCase(
                angle_of_attack = angles_of_attack[angle_index],
                section_model_dict = section_model_dict,
                dynamic = dyn,
                z_symmetry=True
            )

            result_history = sim_case.run()

            print("Number of iterations", result_history[-1].iterations)

            lift_forces = []
            drag_forces = []

            for res in result_history:
                lift_forces.append(res.integrated_forces[0].total.y)
                drag_forces.append(res.integrated_forces[0].total.x)

            lift_forces = np.array(lift_forces)
            drag_forces = np.array(drag_forces)

            t_non_dim = np.linspace(0, 1.0, len(lift_forces))

            mean_indices = np.where(t_non_dim > 0.5) if dyn else -1

            cd[angle_index] = np.mean(drag_forces[mean_indices]) / sim_case.force_factor
            cl[angle_index] = np.mean(lift_forces[mean_indices]) / sim_case.force_factor

        cl_theory = cl_2d / (1 + 2/theoretical_aspect_ratio)
        cd_theory = cd_2d + cl_theory**2 / (np.pi * theoretical_aspect_ratio)

        sim_type = "Dynamic" if dyn else "Quasi-steady"

        ax1.plot(angles_of_attack, cl, label='Lifting line, ' + sim_type)
        ax2.plot(angles_of_attack, cd, label='Lifting line, ' + sim_type)

    ax1.plot(angles_of_attack, cl_2d, label='Section model', color='grey', linestyle='--')
    ax2.plot(angles_of_attack, cd_2d, label='Section model', color='grey', linestyle='--')

    ax1.set_ylim(0, 3.0)
    ax2.set_ylim(0, 0.5)
    
    ax1.set_xlabel("Angle of attack [deg]")
    ax1.set_ylabel("Lift coefficient")

    ax2.set_xlabel("Angle of attack [deg]")
    ax2.set_ylabel("Drag coefficient")


    ax2.legend()

    plt.show()