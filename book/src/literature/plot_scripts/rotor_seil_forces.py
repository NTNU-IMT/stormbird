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
    files = ['prandtl.json', 'sspa_cfd.json']
    folder = Path('../data')

    colors = plt.rcParams['axes.prop_cycle'].by_key()['color']

    max_spin_ratio = 5

    w_plot = 12
    fig = plt.figure(figsize=(w_plot, w_plot / 1.85))
    ax_cl = fig.add_subplot(121)
    ax_cd = fig.add_subplot(122)

    plot_index = 0
    for file in files:
        with open(folder / file, 'r') as f:
            data = json.load(f)

        
        ax_cl.plot(data['spin_ratios'], data['lift_coefficients'], label=data['label'], color=colors[plot_index])

        try:
            ax_cd.plot(data['spin_ratios'], data['drag_coefficients'], color=colors[plot_index])
        except KeyError:
            pass

        plot_index += 1

    spin_ratio_tillig = np.linspace(0, 5, 100)
    cl_tillig = np.zeros_like(spin_ratio_tillig)
    cd_tillig = np.zeros_like(spin_ratio_tillig)

    for i, spin_ratio in enumerate(spin_ratio_tillig):
        cl_tillig[i] = tillig_cl(spin_ratio)
        cd_tillig[i] = tillig_cd(spin_ratio)

    ax_cl.plot(spin_ratio_tillig, cl_tillig, label='Tillig, 2020, empirical model', color=colors[plot_index])
    ax_cd.plot(spin_ratio_tillig, cd_tillig, color=colors[plot_index])

    ax_cl.set_xlabel('Spin ratio')
    ax_cl.set_ylabel('Lift coefficient')

    ax_cl.set_xlim(0, max_spin_ratio)

    ax_cl.legend()

    ax_cd.set_xlabel('Spin ratio')
    ax_cd.set_ylabel('Drag coefficient')

    ax_cd.set_xlim(0, max_spin_ratio)

    plt.tight_layout()

    plt.savefig('../figures/rotor_sail_forces.png', dpi=300, bbox_inches='tight')

    plt.show()

    