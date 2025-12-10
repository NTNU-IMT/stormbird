import argparse

import numpy as np
import matplotlib.pyplot as plt

from setup import simulate_single_case, TEST_SETTINGS

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Run a single case")
    parser.add_argument("--spin-ratio", type=float, default = 4.0, help="Spin ratio")

    args = parser.parse_args()

    w_plot = 16
    fig = plt.figure(figsize=(w_plot, w_plot/2.35))
    ax_circulation = fig.add_subplot(121)
    ax_angle = fig.add_subplot(122)

    for index in range(len(TEST_SETTINGS["solver_types"])):
        solver = TEST_SETTINGS["solver_types"][index]
        max_induced_velocity_ratio = TEST_SETTINGS["max_induced_velocity_ratios"][index]
        smoothing_length = TEST_SETTINGS["smoothing_lengths"][index]
        
        label = solver.name

        if max_induced_velocity_ratio > 0.0:
            label += f", max u_i ratio {max_induced_velocity_ratio:.1f}"
        if smoothing_length > 0.0:
            label += f", smoothing length {smoothing_length:.3f}"

        print("Running simulation case:", label)

        res = simulate_single_case(
            rotor_x_locations = [0.0],
            rotor_y_locations = [0.0],
            spin_ratio = args.spin_ratio,
            solver_type = solver,
            max_induced_velocity_ratio = max_induced_velocity_ratio,
            smoothing_length = smoothing_length
        )[0]

        print('Lift coefficient:', res['cy'])
        print('Drag coefficient:', res['cx'])

        ax_circulation.plot(-res['circulation_strength'], label=label)

        ax_angle.plot(np.degrees(res['angles_of_attack']), label=label)

    ax_circulation.set_xlabel('Line model segment')
    ax_circulation.set_ylabel('Circulation strength')
    
    ax_circulation.set_ylim(0, None)

    ax_angle.set_xlabel('Line model segment')
    ax_angle.set_ylabel('Effective angle of attack [deg]')

    ax_angle.legend()

    plt.show()
