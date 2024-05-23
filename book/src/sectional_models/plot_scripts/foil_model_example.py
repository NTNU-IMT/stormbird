import numpy as np

import matplotlib.pyplot as plt

from pystormbird.section_models import Foil

if __name__ == '__main__':
    foil = Foil()

    n_plot = 200
    angles_of_attack_deg = np.linspace(0, 90, n_plot)
    angles_of_attack = np.radians(angles_of_attack_deg)

    cl = np.zeros(n_plot)
    cd = np.zeros(n_plot)

    for i in range(n_plot):
        cl[i] = foil.lift_coefficient(angles_of_attack[i])

    plt.plot(angles_of_attack_deg, cl)
    plt.savefig('../figures/foil_model_example.png', dpi=300)
    plt.show()