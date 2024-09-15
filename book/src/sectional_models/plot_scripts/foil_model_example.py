import numpy as np

import matplotlib.pyplot as plt

from pystormbird.section_models import Foil

if __name__ == '__main__':
    default_colors = plt.rcParams['axes.prop_cycle'].by_key()['color']

    y_min = -2.0
    y_max = 2.3

    foil = Foil(
        cl_zero_angle = 0.5,
        cl_high_order_factor = 0.0,
        cl_high_order_power = 4.0,
        cl_max_after_stall = 0.95,
        cd_max_after_stall = 1.2,
        mean_positive_stall_angle = np.radians(20),
        mean_negative_stall_angle = np.radians(25),
        stall_range = np.radians(10),
    )

    n_plot = 200
    angles_of_attack_deg = np.linspace(-35, 95, n_plot)
    angles_of_attack = np.radians(angles_of_attack_deg)

    cl = np.zeros(n_plot)
    cd = np.zeros(n_plot)

    for i in range(n_plot):
        cl[i] = foil.lift_coefficient(angles_of_attack[i])
        cd[i] = foil.drag_coefficient(angles_of_attack[i])

    plt.plot(angles_of_attack_deg, cl, color=default_colors[0], label='Lift')
    plt.plot(angles_of_attack_deg, cd, color=default_colors[1], label='Drag')

    plt.fill_between(
        np.degrees([foil.mean_positive_stall_angle - foil.stall_range/2, foil.mean_positive_stall_angle + foil.stall_range/2]), 
        [y_max, y_max], 
        [y_min, y_min], 
        color='grey', alpha=0.1, label='Stall range'
    )

    plt.fill_between(
        np.degrees([-foil.mean_negative_stall_angle - foil.stall_range/2, -foil.mean_negative_stall_angle + foil.stall_range/2]), 
        [y_max, y_max], 
        [y_min, y_min], 
        color='grey', alpha=0.1
    )
    
    plt.plot(np.degrees([foil.mean_positive_stall_angle]*2), [y_min, y_max], '--', color = 'grey', label='Mean stall angle')
    plt.plot(np.degrees([-foil.mean_negative_stall_angle]*2), [y_min, y_max], '--', color = 'grey')

    plt.scatter(0.0, foil.cl_zero_angle, color=default_colors[0])
    plt.text(
        0.0, foil.cl_zero_angle * 1.05, '$C_L(0) = {:.1f}$'.format(foil.cl_zero_angle),
        verticalalignment='bottom', horizontalalignment='right'
    )

    plt.scatter(45, foil.cl_max_after_stall, color=default_colors[0])
    plt.text(
        45, foil.cl_max_after_stall * 1.05, '$C_L(45) = {:.1f}$'.format(foil.cl_max_after_stall),
        verticalalignment='bottom', horizontalalignment='center'
    )

    plt.scatter(90, foil.cd_max_after_stall, color=default_colors[1])
    plt.text(
        90, foil.cd_max_after_stall * 1.05, '$C_D(90) = {:.1f}$'.format(foil.cd_max_after_stall),
        verticalalignment='bottom', horizontalalignment='right'
    )

    plt.xlabel('Angle of attack [deg]')
    plt.ylabel('Force coefficient')

    plt.legend(loc='lower right')

    plt.ylim(y_min, y_max)
    plt.xlim(np.min(angles_of_attack_deg), np.max(angles_of_attack_deg))
    plt.savefig('../figures/foil_model_example.png', dpi=300)
    plt.show()