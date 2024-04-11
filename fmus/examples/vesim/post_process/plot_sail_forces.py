import pandas as pd

import numpy as numpy
import matplotlib.pyplot as plt

import os

from pathlib import Path

if __name__ == '__main__':
    output_path_w_sb = Path('../output/output_with_stormbird')
    output_path_n_sb = Path('../output/output_no_stormbird')

    output_path_list = [output_path_w_sb]

    for output_path in output_path_list:
        all_output_files = os.listdir(output_path)

        sails_files = []
        for f in all_output_files:
            if 'sails' in f and '.csv' in f:
                sails_files.append(f)

        sails_files.sort()

        sails_df = pd.read_csv(output_path / Path(sails_files[0]))

        time = sails_df['Time'].to_numpy()

        surge_force = sails_df['force_x'].to_numpy()
        sway_force  = sails_df['force_y'].to_numpy()

        plt.plot(time, surge_force, label='Surge force')
        plt.plot(time, sway_force, label='Sway force')

    plt.legend()

    plt.show()