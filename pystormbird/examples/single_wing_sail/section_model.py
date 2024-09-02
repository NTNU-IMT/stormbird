'''
This script sets up a foil model to best fit the sectional data extracted from Grad et al. (2014)
'''

import numpy as np

import json

import matplotlib.pyplot as plt

from pystormbird.section_models import Foil

def section_model_dict() -> dict:
    return {
        "cl_initial_slope": 0.92 * 2 * np.pi,
        "cd_zero_angle": 0.01910,
        "mean_positive_stall_angle": np.radians(12.5),
        "stall_range": np.radians(4.0),
        "cd_zero_angle": 0.01910,
        "cd_second_order_factor": 1.0,
        "cd_power_after_stall": 1.4,
        "cd_max_after_stall": 1.4,
    }

if __name__ == '__main__':
    comparison_data = json.load(open("graf_2014_data.json", "r"))

    model = Foil.new_from_string(json.dumps(section_model_dict()))

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
    ax1.plot(
        comparison_data["two_dim_cfd"]["angles_of_attack"], 
        comparison_data["two_dim_cfd"]["lift_coefficients"], 
        'o'
    )

    ax2.plot(alpha_test_deg, cd)
    ax2.plot(
        comparison_data["two_dim_cfd"]["angles_of_attack"], 
        comparison_data["two_dim_cfd"]["drag_coefficients"], 
        'o'
    )

    plt.show()

