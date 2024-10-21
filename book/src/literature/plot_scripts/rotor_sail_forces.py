import matplotlib.pyplot as plt
import json
from pathlib import Path

import numpy as np

def tillig_cl(spin_ratio: float) -> float:
    return (
        - 0.0046 * spin_ratio**5
        + 0.1145 * spin_ratio**4
        - 0.9817 * spin_ratio**3
        + 3.1309 * spin_ratio**2
        - 0.1039 * spin_ratio
    )
    
def tillig_cd(spin_ratio: float) -> float:
    return (
        - 0.0017 * spin_ratio**5
        + 0.0464 * spin_ratio**4
        - 0.4424 * spin_ratio**3
        + 1.7243 * spin_ratio**2
        - 1.6410 * spin_ratio
        + 0.6375
    )

if __name__ == '__main__':
    folder = Path('../data')

    with open(folder / 'rotor_sail_data.json', 'r') as f:
        rotor_sail_data = json.load(f)

    colors = plt.rcParams['axes.prop_cycle'].by_key()['color']

    max_spin_ratio = 5
    max_cl = 12.5

    w_plot = 10
    fig_2d = plt.figure(figsize=(w_plot, w_plot / 1.85))
    fig_3d = plt.figure(figsize=(w_plot, w_plot / 1.85))

    fig_list = [fig_2d, fig_3d]

    dimensions = [2, 3]

    for fig, dim in zip(fig_list, dimensions):
        ax_cl = fig.add_subplot(121)
        ax_cd = fig.add_subplot(122)

        ax_list = [ax_cl, ax_cd]

        plot_index = 0
        for data in rotor_sail_data:
            if data['dimensions'] != dim:
                continue

            ax_cl.plot(data['spin_ratios'], data['lift_coefficients'], label=data['label'], color=colors[plot_index])

            try:
                if dim == 2:
                    ax_cd.plot(data['spin_ratios'], data['drag_coefficients'], label=data['label'], color=colors[plot_index])
                else:
                    ax_cd.plot(data['drag_coefficients'], data['lift_coefficients'], label=data['label'], color=colors[plot_index])
            except KeyError:
                ax_cd.plot([-2, -1], [0.0, 0.0], label=data['label'], color=colors[plot_index])

            plot_index += 1

        if dim == 3:
            spin_ratio_tillig = np.linspace(0, 5, 100)
            cl_tillig = np.zeros_like(spin_ratio_tillig)
            cd_tillig = np.zeros_like(spin_ratio_tillig)

            for i, spin_ratio in enumerate(spin_ratio_tillig):
                cl_tillig[i] = tillig_cl(spin_ratio)
                cd_tillig[i] = tillig_cd(spin_ratio)

            tillig_label = 'Tillig, 2020, empirical model'
            ax_cl.plot(spin_ratio_tillig, cl_tillig, label=tillig_label, color=colors[plot_index])
            ax_cd.plot(cd_tillig, cl_tillig, label=tillig_label, color=colors[plot_index])

            cl_induced = np.linspace(0, max_cl, 100)
            asp_effective = 2 * (30 / 5.0)
            cd_induced = cl_induced**2 / (np.pi * asp_effective)

            ax_cd.plot(cd_induced, cl_induced, color='grey', linestyle='--', label=f'Elliptic wing theory, asp = {asp_effective}')

        ax_cl.set_xlabel('Spin ratio')
        ax_cl.set_ylabel('Lift coefficient')

        ax_cl.set_xlim(0, max_spin_ratio)
        ax_cl.set_ylim(0, max_cl)
    
        ax_cd.legend(loc='lower right', prop={'size': 'small'})

        if dim == 2:
            ax_cd.set_ylabel('Drag coefficient')
            ax_cd.set_xlabel("Spin ratio")
            ax_cd.set_xlim(0, max_spin_ratio)

        else:
            ax_cd.set_xlabel('Drag coefficient')
            ax_cd.set_ylabel('Lift coefficient')
            ax_cd.set_xlim(0, 4.0)
        

        for ax in ax_list:
            ax.grid(True)

        fig.tight_layout()

        fig.savefig(f'../figures/rotor_sail_forces_{dim}d.png', dpi=300, bbox_inches='tight')

    plt.show()

    