import numpy as np

import matplotlib.pyplot as plt

import argparse

from simulation import SimulationCase

if __name__ == '__main__':
    argument_parser = argparse.ArgumentParser()
    argument_parser.add_argument("--angle-of-attack", type=float, default = 0.0, help="Angle of attack in degrees")
    argument_parser.add_argument("--wind-angle", type=float, default = 45.0, help="Wind angle in degrees")

    args = argument_parser.parse_args()

    angles_of_attack = np.arange(0.0, 16, 0.5)

    drag_1 = []
    drag_2 = []
    lift_1 = []
    lift_2 = []

    simulation = SimulationCase(
        angle_of_attack_deg = args.angle_of_attack,
        wind_angle_deg=args.wind_angle,
        write_wake_files=True
    )

    result = simulation.run()

    force_wing_1 = result.integrated_forces[0].total
    force_wing_2 = result.integrated_forces[1].total

    cl1 = force_wing_1.y / simulation.force_factor
    cl2 = force_wing_2.y / simulation.force_factor

    cd1 = force_wing_1.x / simulation.force_factor
    cd2 = force_wing_2.x / simulation.force_factor

    print(f'Cl1: {cl1}')
    print(f'Cl2: {cl2}')

    print(f'Cd1: {cd1}')
    print(f'Cd2: {cd2}')

    ctrl_points_z = []
    for i, ctrl_point in enumerate(result.ctrl_points):
        ctrl_points_z.append(ctrl_point.z)

    ctrl_points_z = np.array(ctrl_points_z)
    circulation_strength = np.array(result.force_input.circulation_strength)

    ctrl_points_z_1 = ctrl_points_z[0: len(ctrl_points_z) // 2]
    ctrl_points_z_2 = ctrl_points_z[len(ctrl_points_z) // 2:]

    circualtion_strength_1 = circulation_strength[0: len(circulation_strength) // 2]
    circualtion_strength_2 = circulation_strength[len(circulation_strength) // 2:]

    ctrl_points_list = [ctrl_points_z_1, ctrl_points_z_2]
    circulation_strength_list = [circualtion_strength_1, circualtion_strength_2]

    w_plot = 12
    fig = plt.figure(figsize=(w_plot, w_plot/1.85))
    ax_circulation = fig.add_subplot(111)

    for z, gamma in zip(ctrl_points_list, circulation_strength_list):
        ax_circulation.plot(z, -gamma)

    plt.show()

        