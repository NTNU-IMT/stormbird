import matplotlib.pyplot as plt
import numpy as np
import pandas as pd

from pathlib import Path

import os

from utils import convert_lists_to_string, find_output_file

if __name__ == '__main__':
    output_folder = Path('../output')

    output_file_path_sails = find_output_file(output_folder, 'wing_sails')

    df_sails = pd.read_csv(output_file_path_sails)

    output_file_path_controllers = find_output_file(output_folder, 'controller')

    df_controllers = pd.read_csv(output_file_path_controllers)

    plt.plot(df_sails['Time'], df_sails['angle_of_attack_measurement_1'])
    plt.plot(df_controllers['Time'], df_controllers['angle_of_attack_estimate_1'])

    plt.show()
