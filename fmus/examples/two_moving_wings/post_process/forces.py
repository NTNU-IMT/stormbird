import matplotlib.pyplot as plt
import numpy as np
import pandas as pd

from pathlib import Path

import os

from utils import convert_lists_to_string, find_output_file

if __name__ == '__main__':
    output_folder = Path('../output')

    output_file_path = find_output_file(output_folder, 'wing_sails')
    output_file_path = convert_lists_to_string(output_file_path)

    df = pd.read_csv(output_file_path)

    time = df['Time']
    thrust = df['force_x']
    side_force = df['force_y']

    nr_rows = len(thrust)

    plt.plot(time, thrust)
    plt.plot(time, side_force)

    plt.show()
