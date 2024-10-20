import numpy as np

import matplotlib.pyplot as plt

from simulation import SimulationCase

if __name__ == '__main__':
    angles_of_attack = np.arange(0.0, 16, 0.5)

    drag_1 = []
    drag_2 = []
    lift_1 = []
    lift_2 = []

    for angle in angles_of_attack:
        simulation = SimulationCase(
            angle_of_attack_deg = angle,
            wind_angle_deg=45.0,
            write_wake_files=True
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

    ax_lift.plot(angles_of_attack, lift_1, label='Wing 1')
    ax_lift.plot(angles_of_attack, lift_2, label='Wing 2')

    ax_drag.plot(angles_of_attack, drag_1, label='Wing 1')
    ax_drag.plot(angles_of_attack, drag_2, label='Wing 2')

    ax_lift.set_xlabel('Angle of attack [deg]')
    ax_drag.set_xlabel('Angle of attack [deg]')

    ax_lift.set_ylabel('Lift Coefficient')
    ax_drag.set_ylabel('Drag Coefficient')

    ax_drag.set_ylim(0, 0.14)
    ax_lift.set_ylim(0, 1.2)

    ax_lift.legend()

    plt.show()

        