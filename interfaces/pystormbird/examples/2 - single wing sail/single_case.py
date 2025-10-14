import numpy as np
import matplotlib.pyplot as plt

import time

import sys

import argparse

from simulation import SimulationCase, SolverType

section_model_path = '../1 - section models'

if section_model_path not in sys.path:
    sys.path.append(section_model_path)

import naca_0012_Graf2014
import manual_multi_element_foil

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Run a single case")
    parser.add_argument("--angle-of-attack", type=float, default = 0.0, help="Angle of attack in degrees")
    parser.add_argument("--multi-element", action="store_true", help="Use a multi-element foil model")
    parser.add_argument("--flap-angle", type=float, default = 0.0, help="Flap angle in degrees")
    parser.add_argument("--write-wake-files", action="store_true", help="Write wake files")

    args = parser.parse_args()

    if args.multi_element:
        section_model = manual_multi_element_foil.get_section_model_setup(flap_angle=np.radians(args.flap_angle))
    else:
        foil_tuner = naca_0012_Graf2014.get_tuned_foil_tuner()

        section_model = foil_tuner.get_section_model_setup()

    w_plot = 16
    fig = plt.figure(figsize=(w_plot, w_plot/3.0))
    ax1 = fig.add_subplot(131)
    ax2 = fig.add_subplot(132)
    ax3 = fig.add_subplot(133)

    solver_types = [SolverType.Linearized, SolverType.Linearized, SolverType.SimpleIterative, SolverType.SimpleIterative]
    dynamic = [False, True, False, True]

    for dyn, solver in zip(dynamic, solver_types):
        sim_label = "Dynamic" if dyn else "Steady"
        sim_label += " - " + solver.name
        
        print("Running simulation case:", sim_label)

        write_wake_files = args.write_wake_files if dyn else False

        sim_case = SimulationCase(
            angle_of_attack = args.angle_of_attack,
            section_model = section_model,
            solver_type = solver,
            dynamic = dyn,
            z_symmetry=True
        )

        start_time = time.time()

        result_history = sim_case.run()

        end_time = time.time()

        elapsed_time = end_time - start_time

        print('Elapsed time:', elapsed_time)
        #print('Time speed up', sim_case.end_time / elapsed_time)

        cd = np.zeros(len(result_history))
        cl = np.zeros(len(result_history))

        for i in range(len(result_history)):
            force = result_history[i].integrated_forces[0].total

            cd[i] = force.x / sim_case.force_factor
            cl[i] = force.y / sim_case.force_factor

        print('Last lift coefficient:', cl[-1])
        print('Last drag coefficient:', cd[-1])

        if solver == SolverType.SimpleIterative:
            linestyle='--'
        else:
            linestyle='-'

        circulation_strength = np.array(result_history[-1].force_input.circulation_strength)
        angles_of_attack     = np.array(result_history[-1].force_input.angles_of_attack)

        if len(cl) > 1:
            ax1.plot(cl)
        else:
            ax1.plot([0, 1], [cl[0], cl[0]])

        ax2.plot(-circulation_strength, label=sim_label, linestyle=linestyle)

        ax3.plot(np.degrees(angles_of_attack), label=sim_label, linestyle=linestyle)

    ax3.plot([0, len(angles_of_attack)], [args.angle_of_attack, args.angle_of_attack], 'k--', label='Geometric angle of attack')

    ax1.set_xlabel('Time step')
    ax1.set_ylabel('Lift coefficient')

    ax2.set_xlabel('Line model segment')
    ax2.set_ylabel('Circulation strength')

    ax3.set_xlabel('Line model segment')
    ax3.set_ylabel('Effective angle of attack [deg]')

    ax3.legend()

    plt.show()
