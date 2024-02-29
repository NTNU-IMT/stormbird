import pandas as pd

import os

import matplotlib.pyplot as plt

if __name__ == "__main__":
    result_file_list = os.listdir("../output/")

    rotor_area = 5.0 * 30.0
    velocity   = 15.70796

    force_factor = 0.5 * rotor_area * velocity**2

    dynamic_results = []
    for result_file in result_file_list:
        if result_file.endswith(".csv") and 'dynamic' in result_file:
            dynamic_results.append(result_file)

    # Sort the results, which will order them from oldes to newest (since the file names contain timestamps)
    dynamic_results.sort()

    # Load the newest results and plot them
    dynamic_data = pd.read_csv("../output/" + dynamic_results[-1])

    force_x = dynamic_data['force_x'].to_numpy() / force_factor
    force_y = dynamic_data['force_y'].to_numpy() / force_factor

    print('Last force in x-direction', force_x[-1])
    print('Last force in y-direction', force_y[-1])

    w_plot = 16
    h_plot = w_plot / 2.35

    fig = plt.figure(figsize=(w_plot, h_plot))
    ax1 = fig.add_subplot(121)
    ax2 = fig.add_subplot(122)

    plt.sca(ax1)
    plt.plot(dynamic_data['Time'], force_x)
    plt.xlabel('Time (s)')
    plt.ylabel('Force in x-direction (N)')

    plt.sca(ax2)
    plt.plot(dynamic_data['Time'], force_y)

    plt.xlabel('Time (s)')
    plt.ylabel('Force in y-direction (N)')

    plt.tight_layout()
    
    plt.savefig('force_comparison.png', dpi=300, bbox_inches='tight')

    plt.show()