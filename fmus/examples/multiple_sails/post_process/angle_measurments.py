import matplotlib.pyplot as plt
import numpy as np
import pandas as pd

import ast

from pathlib import Path

import os

def convert_lists_to_string(file_path: Path) -> Path:
    with open(file_path, 'r') as f:
        lines = f.readlines()

    new_lines = []
    for line in lines:
        if '[' in line:
            line = line.replace("[", "'[").replace("]", "]'")

        new_lines.append(line)

    out_path = file_path.parent / (file_path.stem + '_stringified' + file_path.suffix)

    with open(out_path, 'w') as f:
        f.writelines(new_lines)

    return out_path

if __name__ == '__main__':
    output_folder = Path('../output')

    files_in_output = os.listdir(output_folder)

    output_file_path = Path()
    for file in files_in_output:
        if 'wing_sails' in file and '.csv' in file:
            output_file_path = output_folder / file

    output_file_path = convert_lists_to_string(output_file_path)

    df = pd.read_csv(output_file_path, engine='python')

    angles_of_attack = df['angles_of_attack_measurment']

    nr_rows = len(angles_of_attack)

    angle_1 = np.zeros(nr_rows)
    angle_2 = np.zeros(nr_rows)
    for i in range(nr_rows):
        if not pd.isna(angles_of_attack[i]):
            angle_string = angles_of_attack[i].split(',')

            angle_1[i] = float(angle_string[0][2:])
            angle_2[i] = float(angle_string[1][0:-2])

    plt.plot(df['Time'], angle_1)
    plt.plot(df['Time'], angle_2)

    plt.show()
