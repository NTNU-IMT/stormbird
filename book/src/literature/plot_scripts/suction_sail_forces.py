import matplotlib.pyplot as plt
import json
from pathlib import Path

import numpy as np

if __name__ == '__main__':
    folder = Path('../data')

    with open(folder / 'suction_sail_data.json', 'r') as f:
        rotor_sail_data = json.load(f)

    colors = plt.rcParams['axes.prop_cycle'].by_key()['color']

    max_angle_of_attack = 40
    max_cl = 9.0

    w_plot = 10
    fig = plt.figure(figsize=(w_plot, w_plot / 1.85))
    ax_cl = fig.add_subplot(121)
    ax_cd = fig.add_subplot(122)

    ax_list = [ax_cl, ax_cd]

    plot_index = 0
    for data in rotor_sail_data:
        ax_cl.plot(data['angles_of_attack'], data['lift_coefficients'], label=data['label'], color=colors[plot_index])

        try:
            ax_cd.plot(data['drag_coefficients'], data['lift_coefficients'], label=data['label'], color=colors[plot_index])
        except KeyError:
            ax_cd.plot([-2, -1], [0.0, 0.0], label=data['label'], color=colors[plot_index])

        plot_index += 1


    cl_induced = np.linspace(0, max_cl, 100)
    asp_effective = 2 * 4.0
    cd_induced = cl_induced**2 / (np.pi * asp_effective)

    ax_cd.plot(cd_induced, cl_induced, color='grey', linestyle='--', label=f'Elliptic wing theory, asp = {asp_effective}')

    ax_cl.set_xlabel('Angle of attack [deg]')

    ax_cl.set_xlim(0, max_angle_of_attack)
    ax_cd.set_xlim(0, 2.0)

    ax_cd.legend(loc='lower right', prop={'size': 'small'})

    ax_cd.set_xlabel('Drag coefficient')
    

    for ax in ax_list:
        ax.set_ylim(0, max_cl)
        ax.set_ylabel('Lift coefficient')
        ax.grid(True)

    plt.tight_layout()

    plt.savefig('../figures/suction_sail_forces.png', dpi=300, bbox_inches='tight')

    plt.show()

    