'''
This script sets up a foil model to best fit the sectional data extracted from Grad et al. (2014)
'''

import numpy as np
import scipy.optimize as opt

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

def cl_section_model(x):
    return Foil(
        cl_initial_slope          = x[0],
        cl_max_after_stall        = x[1],
        mean_positive_stall_angle = x[2],
        stall_range               = x[3]
    )

def cd_section_model(x, cl_x):
    return Foil(
        cl_initial_slope          = cl_x[0],
        cl_max_after_stall        = cl_x[1],
        mean_positive_stall_angle = cl_x[2],
        stall_range               = cl_x[3],
        cd_zero_angle             = x[0],
        cd_second_order_factor    = x[1],
        cd_power_after_stall      = x[2],
        cd_max_after_stall        = x[3]
    )

def cl_objective_function(x):
    comparison_data = json.load(open("graf_2014_data.json", "r"))['two_dim_cfd']

    alpha_comp = np.radians(comparison_data["angles_of_attack"])
    cl_comp = comparison_data["lift_coefficients"]

    model = cl_section_model(x)

    cl_model = np.zeros(len(alpha_comp))

    for i in range(len(alpha_comp)):
        cl_model[i] = model.lift_coefficient(alpha_comp[i])

    return np.sum((cl_model - cl_comp) ** 2)


def cd_objective_function(x, cl_x):
    comparison_data = json.load(open("graf_2014_data.json", "r"))['two_dim_cfd']

    alpha_comp = np.radians(comparison_data["angles_of_attack"])
    cd_comp = comparison_data["drag_coefficients"]

    model = cd_section_model(x, cl_x)

    cd_model = np.zeros(len(alpha_comp))

    for i in range(len(alpha_comp)):
        cd_model[i] = model.drag_coefficient(alpha_comp[i])

    return np.sum((cd_model - cd_comp) ** 2)

def optimized_model():
    cl_x = opt.minimize(
        cl_objective_function, 
        [2 * np.pi, 0.01910, np.radians(14.0), np.radians(6.0)]
    ).x

    cd_x = opt.minimize(
        cd_objective_function, 
        [0.01910, 1.0, 1.4, 1.4],
        args=(cl_x)
    ).x

    return cd_section_model(cd_x, cl_x)

if __name__ == '__main__':
    comparison_data = json.load(open("graf_2014_data.json", "r"))

    model = optimized_model()

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

