'''
This script sets up a foil model to best fit the sectional data extracted from Grad et al. (2014)
'''

import numpy as np
import scipy.optimize as opt

import json

import matplotlib.pyplot as plt

from pystormbird.section_models import Foil

class SingleElementFoilTuner():
    def __init__(self):
        self.model = Foil()

    def set_cl_parameters(self, x):
        self.model.cl_initial_slope = x[0]
        self.model.cl_max_after_stall = x[1]
        self.model.mean_positive_stall_angle = x[2]
        self.model.stall_range = x[3]

    def set_cd_parameters(self, x):
        self.model.cd_zero_angle = x[0]
        self.model.cd_second_order_factor = x[1]
        self.model.cd_power_after_stall = x[2]
        self.model.cd_max_after_stall = x[3]

    def cl_objective_function(self, x):
        comparison_data = json.load(open("graf_2014_data.json", "r"))['two_dim_cfd']

        alpha_comp = np.radians(comparison_data["angles_of_attack"])
        cl_comp = comparison_data["lift_coefficients"]

        self.set_cl_parameters(x)

        cl_model = np.zeros(len(alpha_comp))

        for i in range(len(alpha_comp)):
            cl_model[i] = self.model.lift_coefficient(alpha_comp[i])

        return np.sum((cl_model - cl_comp) ** 2)

    def cd_objective_function(self, x):
        comparison_data = json.load(open("graf_2014_data.json", "r"))['two_dim_cfd']

        alpha_comp = np.radians(comparison_data["angles_of_attack"])
        cd_comp = comparison_data["drag_coefficients"]

        self.set_cd_parameters(x)

        cd_model = np.zeros(len(alpha_comp))

        for i in range(len(alpha_comp)):
            cd_model[i] = self.model.drag_coefficient(alpha_comp[i])

        return np.sum((cd_model - cd_comp) ** 2)

def get_foil_model():
    foil = SingleElementFoilTuner()

    cl_x = opt.minimize(
        foil.cl_objective_function, 
        [2 * np.pi, 0.01910, np.radians(14.0), np.radians(6.0)]
    ).x

    foil.set_cl_parameters(cl_x)

    cd_x = opt.minimize(
        foil.cd_objective_function, 
        [0.01910, 1.0, 1.4, 1.4],
    ).x

    foil.set_cd_parameters(cd_x)

    return foil.model

class TestClass():
    def __init__(self):
        self.a = 1.0
        self.b = 2.0

if __name__ == '__main__':
    comparison_data = json.load(open("graf_2014_data.json", "r"))

    model = tuned_model()

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

