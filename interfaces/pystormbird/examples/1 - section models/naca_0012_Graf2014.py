import numpy as np
import json
import matplotlib.pyplot as plt
from foil_tuner import FoilTuner

def comparison_data():
    comparison_data = json.load(open("../comparison_data/graf_2014_data.json", "r"))['two_dim_cfd']

    angles_of_attack_data = comparison_data["angles_of_attack"]
    cd_data = comparison_data["drag_coefficients"]
    cl_data = comparison_data["lift_coefficients"]

    return angles_of_attack_data, cd_data, cl_data

def get_foil_model():
    angles_of_attack_data, cd_data, cl_data = comparison_data()

    foil_tuner = FoilTuner(
        angles_of_attack_data = np.radians(angles_of_attack_data),
        cd_data = cd_data,
        cl_data = cl_data
    )

    return foil_tuner.get_tuned_model()

if __name__ == '__main__':
    angles_of_attack_data, cd_data, cl_data = comparison_data()

    model = get_foil_model()

    n_test = 100
    alpha_test_deg = np.linspace(0, 20, n_test)
    alpha_test = np.radians(alpha_test_deg)

    cl = np.zeros(n_test)
    cd = np.zeros(n_test)

    for i in range(n_test):
        cl[i] = model.lift_coefficient(alpha_test[i])
        cd[i] = model.drag_coefficient(alpha_test[i])

    w_plot = 18
    h_plot = w_plot / 2.35

    fig = plt.figure(figsize=(w_plot, h_plot))

    ax1 = fig.add_subplot(121)
    ax2 = fig.add_subplot(122)

    ax1.plot(alpha_test_deg, cl)
    ax1.plot(angles_of_attack_data, cl_data, 'o')

    ax2.plot(alpha_test_deg, cd)
    ax2.plot(angles_of_attack_data, cd_data, 'o')

    plt.show()