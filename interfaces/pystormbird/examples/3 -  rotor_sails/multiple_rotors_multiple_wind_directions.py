import numpy as np
import matplotlib.pyplot as plt

from setup import simulate_single_case, TEST_SETTINGS

if __name__ == "__main__":
    default_colors = plt.rcParams['axes.prop_cycle'].by_key()['color']
    
    spin_ratio = 3.0
    
    n_directions = 100
    wind_directions_deg = np.linspace(0.0, 180, n_directions)

    w_plot = 18
    h_plot = w_plot / 2.35
    fig = plt.figure(figsize=(w_plot, h_plot))

    ax1 = fig.add_subplot(121)
    ax2 = fig.add_subplot(122)

    for index in range(len(TEST_SETTINGS["solver_types"])):
        solver = TEST_SETTINGS["solver_types"][index]
        max_induced_velocity_ratio = TEST_SETTINGS["max_induced_velocity_ratios"][index]
        smoothing_length = TEST_SETTINGS["smoothing_lengths"][index]
        
        label = solver.name

        if max_induced_velocity_ratio > 0.0:
            label += f", max u_i ratio {max_induced_velocity_ratio:.1f}"
        if smoothing_length > 0.0:
            label += f", smoothing length {smoothing_length:.3f}"

        print()
        print(label)

        cx1 = np.zeros(n_directions)
        cy1 = np.zeros(n_directions)
        
        cx2 = np.zeros(n_directions)
        cy2 = np.zeros(n_directions)

        for dir_index in range(n_directions):
            print("Testing wind direction: ", wind_directions_deg[dir_index])

            res = simulate_single_case(
                rotor_x_locations = [-10.0, 10.0],
                rotor_y_locations = [0.0, 0.0],
                spin_ratio = spin_ratio,
                solver_type = solver,
                max_induced_velocity_ratio = max_induced_velocity_ratio,
                smoothing_length = smoothing_length,
                wind_direction_deg=wind_directions_deg[dir_index]
            )

            cx1[dir_index] = -res[0]['cx']
            cy1[dir_index] = res[0]['cy']
            
            cx2[dir_index] = -res[1]['cx']
            cy2[dir_index] = res[1]['cy']

        ax1.plot(wind_directions_deg, cx1, label='Rotor 1, ' + label, color=default_colors[index])
        ax2.plot(wind_directions_deg, cy1, label='Rotor 1, ' + label, color=default_colors[index])
        
        ax1.plot(wind_directions_deg, cx2, '--', label='Rotor 2, ' + label, color=default_colors[index])
        ax2.plot(wind_directions_deg, cy2, '--', label='Rotor 2, ' + label, color=default_colors[index])
    
        
    ax1.set_xlabel("Wind direction [deg]")
    ax1.set_ylabel("Thrust coefficient")

    ax2.set_xlabel("Wind direction [deg]")
    ax2.set_ylabel("Side force coefficient")

    ax1.legend()
    plt.show()
