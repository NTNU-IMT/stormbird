import numpy as np
import matplotlib.pyplot as plt

import sys

import argparse

from simulation import SimulationCase, SimulationMode, TestCase

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
        foil_model = manual_multi_element_foil.get_foil_model()

        foil_model.set_internal_state(np.radians(args.flap_angle))

        section_model_dict = {
            "VaryingFoil": foil_model.__dict__
        }
    else:
        foil_model = naca_0012_Graf2014.get_foil_model()

        section_model_dict = {
            "Foil": foil_model.__dict__
        }

    cases = [
        TestCase.RAW_SIMULATION, 
        TestCase.PRESCRIBED_CIRCULATION, 
        TestCase.INITIALIZED_SIMULATION,
        TestCase.SMOOTHED,
        TestCase.SMOOTHED
    ]

    modes = [
        SimulationMode.STATIC,
        SimulationMode.STATIC,
        SimulationMode.STATIC,
        SimulationMode.STATIC,
        SimulationMode.DYNAMIC
    ]

    w_plot = 16
    fig = plt.figure(figsize=(w_plot, w_plot/3.0))
    ax1 = fig.add_subplot(131)
    ax2 = fig.add_subplot(132)
    ax3 = fig.add_subplot(133)

    for case, mode in zip(cases, modes):
        print()
        print(case.to_string())

        write_wake_files = args.write_wake_files if mode == SimulationMode.DYNAMIC else False

        sim_case = SimulationCase(
            angle_of_attack = args.angle_of_attack,
            section_model_dict = section_model_dict,
            simulation_mode = mode,
            prescribed_circulation = case.prescribed_circulation,
            prescribed_initialization = case.prescribed_initialization,
            smoothing_length=case.smoothing_length,
            z_symmetry=True,
            write_wake_files=write_wake_files
        )

        result_history = sim_case.run()

        cd = np.zeros(len(result_history))
        cl = np.zeros(len(result_history))

        for i in range(len(result_history)):
            force = result_history[i].integrated_forces[0].total

            cd[i] = force.x / sim_case.force_factor
            cl[i] = force.y / sim_case.force_factor

        print('Last lift coefficient:', cl[-1])
        print('Last drag coefficient:', cd[-1])

        circulation_strength = np.array(result_history[-1].force_input.circulation_strength)
        angles_of_attack     = np.array(result_history[-1].force_input.angles_of_attack)

        if len(cl) > 1:
            ax1.plot(cl)
        else:
            ax1.plot([0, 1], [cl[0], cl[0]])

        ax2.plot(-circulation_strength, label=case.to_string() + ', ' + mode.to_string())

        ax3.plot(np.degrees(angles_of_attack), label=case.to_string() + ', ' + mode.to_string())

    ax3.plot([0, len(angles_of_attack)], [args.angle_of_attack, args.angle_of_attack], 'k--', label='Geometric angle of attack')

    ax1.set_xlabel('Time step')
    ax1.set_ylabel('Lift coefficient')

    ax2.set_xlabel('Line model segment')
    ax2.set_ylabel('Circulation strength')

    ax3.set_xlabel('Line model segment')
    ax3.set_ylabel('Effective angle of attack [deg]')

    ax3.legend()

    plt.show()
