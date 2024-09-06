import json

import numpy as np
import matplotlib.pyplot as plt

from pystormbird.section_models import VaryingFoil

def get_foil_dict(flap_angle: float = 0.0):
    '''
    Simplified setup of a multi-element foil model. Currently, the data is only rough values. It is 
    just a placeholder for the real data that will be used in the future. In practice, the parameters
    in the foil model must be "fitted" to the input data, for instance by using a optimization 
    framework and least squares fitting. This available in SciPy. Will set this up when we have real
    data available.
    '''

    flap_angles = np.radians([0, 5, 10, 15])

    cl_zero_angle = np.array([0.0, 0.3454, 0.7450, 1.0352])
    mean_stall_angle = np.radians([20.0, 19.0 , 17.8, 16.5])

    cd_zero_angle = np.array([0.0101, 0.0154, 0.0328, 0.0542])
    cd_second_order_factor = np.array([0.6, 0.9, 1.2, 1.5])

    foils_data = []
    for i_flap in range(len(flap_angles)):
        foils_data.append(
            {
                "cl_zero_angle": cl_zero_angle[i_flap],
                "cd_zero_angle": cd_zero_angle[i_flap],
                "cd_second_order_factor": cd_second_order_factor[i_flap],
                "mean_positive_stall_angle": mean_stall_angle[i_flap]
            }
        )

    foil_dict = {}

    foil_dict["internal_state_data"] = flap_angles.tolist()
    foil_dict["foils_data"] = foils_data
    foil_dict["current_internal_state"] = flap_angle

    return foil_dict

def get_foil_model():
    '''
    Converts a foil dict to a VaryingFoil model to get access to internal functions for lift and 
    drag coefficients.
    '''

    foil_dict = get_foil_dict()

    input_str = json.dumps(foil_dict)

    return VaryingFoil(input_str)


if __name__ == '__main__':
    '''
    Test the output from the foil model.
    '''

    foil = get_foil_model()

    w_plot = 12

    fig = plt.figure(figsize=(w_plot, w_plot / 2.35))

    ax1 = fig.add_subplot(121)
    ax2 = fig.add_subplot(122)

    flap_angles_to_test = np.radians([0, 5, 10, 15])

    for flap_angle in flap_angles_to_test:
        foil.set_internal_state(flap_angle)

        n_test = 100
        angles_to_test = np.radians(np.linspace(-5, 20, n_test))

        cl = np.zeros(n_test)
        cd = np.zeros(n_test)

        for i in range(n_test):
            cl[i] = foil.lift_coefficient(angles_to_test[i])
            cd[i] = foil.drag_coefficient(angles_to_test[i])

        
        plt.sca(ax1)
        plt.plot(
            np.degrees(angles_to_test), 
            cl, 
            label="Flap angle = " + str(np.round(np.degrees(flap_angle), 1)) + " deg"
        )

        plt.sca(ax2)
        plt.plot(np.degrees(angles_to_test), cd)

    plt.sca(ax1)
    plt.xlabel("Angle of attack [deg]")
    plt.ylabel("Lift coefficient")

    plt.legend(loc=4)

    plt.sca(ax2)
    plt.xlabel("Angle of attack [deg]")
    plt.ylabel("Drag coefficient")

    plt.show()


    





