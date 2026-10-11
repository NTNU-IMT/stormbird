import numpy as np

import matplotlib.pyplot as plt

import argparse

import time

from simulation import SimulationCase

DEFAULT_COLORS = plt.rcParams['axes.prop_cycle'].by_key()['color']

if __name__ == '__main__':
    argument_parser = argparse.ArgumentParser()
    argument_parser.add_argument(
        "--angle-of-attack", type=float, default = 10.0, help="Angle of attack in degrees"
    )
    argument_parser.add_argument(
        "--wind-angle", type=float, default = 45.0, help="Wind angle in degrees"
    )
    argument_parser.add_argument(
        "--nr-panels-per-line-element", type=float, default=50, help="Number of panels per line element making up the wings"
    )

    args = argument_parser.parse_args()

    w_plot = 12
    fig = plt.figure(figsize=(w_plot, w_plot/2.35))
    ax_circ = fig.add_subplot(121)
    ax_force = fig.add_subplot(122)

    dynamic_list = [True, True, False]
    dynamic_shape_list = [False, True, True]
    line_style = ["-", "--", "-."]
    labels = ["Dynamic, fixed wake", "Dynamic, dynamic wake", "Steady, dynamic wake"]
    export_wake = [False, False, True]

    for case_index, (dynamic, dynamic_shape) in enumerate(zip(dynamic_list, dynamic_shape_list)):
        print(labels[case_index])
        simulation = SimulationCase(
            angle_of_attack_deg = args.angle_of_attack,
            wind_angle_deg = args.wind_angle,
            dynamic = dynamic,
            dynamic_shape = dynamic_shape,
            export_wake = export_wake[case_index],
            nr_panels_per_line_element=args.nr_panels_per_line_element
        )

        start_time = time.time()
        results = simulation.run()
        end_time = time.time()

        print("Time to run simulation", end_time - start_time)

        t = []
        cd1 = []
        cd2 = []
        cl1 = []
        cl2 = []

        t_local = 0.0
        for res in results:
            t.append(t_local)
            force_wing_1 = res.integrated_forces[0].total
            force_wing_2 = res.integrated_forces[1].total
    
            cl1.append(force_wing_1[1] / simulation.force_factor)
            cl2.append(force_wing_2[1] / simulation.force_factor)
        
            cd1.append(force_wing_1[0] / simulation.force_factor)
            cd2.append(force_wing_2[0] / simulation.force_factor)

            t_local += 1.0

        ax_force.plot(t, cl1, line_style[case_index], color=DEFAULT_COLORS[0], label="CL1 " + labels[case_index])
        ax_force.plot(t, cl2, line_style[case_index], color=DEFAULT_COLORS[1], label="CL2 " + labels[case_index])

        print("Last CL1", cl1[-1])
        print("Last CL2", cl2[-1])

        last_result = results[-1]
    
        ctrl_points_z = []
        for i, ctrl_point in enumerate(last_result.ctrl_points):
            ctrl_points_z.append(ctrl_point[2])
    
        ctrl_points_z = np.array(ctrl_points_z)
        circulation_strength = np.array(last_result.force_input.circulation_strength)
    
        ctrl_points_z_1 = ctrl_points_z[0: len(ctrl_points_z) // 2]
        ctrl_points_z_2 = ctrl_points_z[len(ctrl_points_z) // 2:]
    
        circualtion_strength_1 = circulation_strength[0: len(circulation_strength) // 2]
        circualtion_strength_2 = circulation_strength[len(circulation_strength) // 2:]
    
        ctrl_points_list = [ctrl_points_z_1, ctrl_points_z_2]
        circulation_strength_list = [circualtion_strength_1, circualtion_strength_2]
    
        for wing_index, (z, gamma) in enumerate(zip(
            ctrl_points_list, 
            circulation_strength_list, 
        )):
            ax_circ.plot(z, -gamma, line_style[case_index], color=DEFAULT_COLORS[wing_index])

        print()

    ax_circ.set_xlabel("z coordinate")
    ax_circ.set_ylabel("Circulation")

    ax_force.set_xlabel("Iteration")
    ax_force.set_ylabel("Lift coefficient")

    ax_force.legend()

    plt.show()

        