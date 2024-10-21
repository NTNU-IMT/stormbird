import numpy as np
import json
import matplotlib.pyplot as plt

import argparse 

from simulation import SimulationCase

if __name__ == '__main__':
    argument_parser = argparse.ArgumentParser()
    argument_parser.add_argument('--dynamic', action='store_true')

    args = argument_parser.parse_args()

    angles_of_attack = np.arange(0.0, 16.5, 0.5)

    with open('cfd_data.json', 'r') as f:
        cfd_data = json.load(f)

    drag_1 = []
    drag_2 = []
    lift_1 = []
    lift_2 = []

    for angle in angles_of_attack:
        simulation = SimulationCase(
            angle_of_attack_deg = angle,
            wind_angle_deg=45.0,
            dynamic = args.dynamic
        )

        result = simulation.run()

        force_wing_1 = result.integrated_forces[0].total
        force_wing_2 = result.integrated_forces[1].total

        drag_1.append(force_wing_1.x / simulation.force_factor)
        drag_2.append(force_wing_2.x / simulation.force_factor)
        lift_1.append(force_wing_1.y / simulation.force_factor)
        lift_2.append(force_wing_2.y / simulation.force_factor)
                      
    w_plot = 12
    fig = plt.figure(figsize=(w_plot, w_plot/1.85))
    ax_lift = fig.add_subplot(122)
    ax_drag = fig.add_subplot(121)

    ax_lift.plot(angles_of_attack, lift_1, label='Port sail, lifting line')
    ax_lift.plot(angles_of_attack, lift_2, label='Starboard sail, lifting line')

    ax_drag.plot(angles_of_attack, drag_1, label='Port sail, lifting line')
    ax_drag.plot(angles_of_attack, drag_2, label='Starboard sail, lifting line')

    for data in cfd_data:
        ax_lift.scatter(data['angles_of_attack'], data['lift_coefficients'], label=data['label'])
        ax_drag.scatter(data['angles_of_attack'], data['drag_coefficients'], label=data['label'])

    ax_lift.set_xlabel('Angle of attack [deg]')
    ax_drag.set_xlabel('Angle of attack [deg]')

    ax_lift.set_ylabel('Lift Coefficient')
    ax_drag.set_ylabel('Drag Coefficient')

    ax_drag.set_xlim(0, 16)
    ax_lift.set_xlim(0, 16)
    ax_drag.set_ylim(0, 0.14)
    ax_lift.set_ylim(0, 1.2)

    ax_lift.legend()

    plt.show()

        