import pandas as pd

import os

import matplotlib.pyplot as plt

if __name__ == "__main__":
    result_file_list = os.listdir("../output/")

    # Filter file names based on the type of results
    quasi_steady_results = []
    for result_file in result_file_list:
        if result_file.endswith(".csv") and 'quasi_steady' in result_file:
            quasi_steady_results.append(result_file)

    dynamic_results = []
    for result_file in result_file_list:
        if result_file.endswith(".csv") and 'dynamic' in result_file:
            dynamic_results.append(result_file)

    # Sort the results, which will order them from oldes to newest (since the file names contain timestamps)
    quasi_steady_results.sort()
    dynamic_results.sort()

    # Load the newest results and plot them
    quasi_steady_data = pd.read_csv("../output/" + quasi_steady_results[-1])
    dynamic_data = pd.read_csv("../output/" + dynamic_results[-1])

    w_plot = 16
    h_plot = w_plot / 2.35

    fig = plt.figure(figsize=(w_plot, h_plot))
    ax1 = fig.add_subplot(121)
    ax2 = fig.add_subplot(122)

    plt.sca(ax1)

    plt.plot(quasi_steady_data['Time'], quasi_steady_data['force_x'], label='Quasi-steady')
    plt.plot(dynamic_data['Time'], dynamic_data['force_x'], label='Dynamic wake')
    plt.xlabel('Time (s)')
    plt.ylabel('Force in x-direction (N)')

    plt.sca(ax2)

    plt.plot(quasi_steady_data['Time'], quasi_steady_data['force_y'], label='Quasi-steady')
    plt.plot(dynamic_data['Time'], dynamic_data['force_y'], label='Dynamic wake')

    plt.xlabel('Time (s)')
    plt.ylabel('Force in y-direction (N)')
    plt.legend()

    plt.tight_layout()
    
    plt.savefig('force_comparison.png', dpi=300, bbox_inches='tight')

    plt.show()