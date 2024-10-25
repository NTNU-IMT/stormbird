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

    angles_of_attack = df['angles_of_attack_measurment']

    nr_rows = len(angles_of_attack)

    angles_of_attack_sail_1 = np.zeros(nr_rows)
    angles_of_attack_sail_2 = np.zeros(nr_rows)
    for i in range(nr_rows):
        if not pd.isna(angles_of_attack[i]):
            angle_string = angles_of_attack[i].strip(']').strip('[').split(',')

            angles_of_attack_sail_1[i] = float(angle_string[0])
            angles_of_attack_sail_2[i] = float(angle_string[1])

    plt.plot(df['Time'], angles_of_attack_sail_1)
    plt.plot(df['Time'], angles_of_attack_sail_2)

    plt.show()
