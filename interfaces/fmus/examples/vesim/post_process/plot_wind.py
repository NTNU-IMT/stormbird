import pandas as pd

import numpy as np
import matplotlib.pyplot as plt

import os

from pathlib import Path

import argparse

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description='Plot results.')
    parser.add_argument('--with-stormbird', action='store_true', help='Run with stormbird.')

    args = parser.parse_args()

    output_path_w_sb = Path('../output_with_stormbird')
    output_path_n_sb = Path('../output_no_stormbird')

    output_path_list = [output_path_w_sb]

    for output_path in output_path_list:
        all_output_files = os.listdir(output_path)

        sobc_files = []
        for f in all_output_files:
            if 'SOBC1' in f and '.csv' in f:
                sobc_files.append(f)

        sobc_files.sort()

        sobc1_df = pd.read_csv(output_path / Path(sobc_files[0]))

        time = sobc1_df['Time'].to_numpy()

        velocity_relative  = sobc1_df['wind_velocity_relative_to_ship'].to_numpy()
        direction_relative = sobc1_df['wind_direction_relative_to_ship'].to_numpy()
        
        velocity_global  = sobc1_df['global_wind_vel'].to_numpy()
        direction_global = sobc1_df['global_wind_dir'].to_numpy()
        
        w_plot = 16
        h_plot = w_plot / 2.35
        fig = plt.figure(figsize=(w_plot, h_plot))

        ax1 = fig.add_subplot(121)
        ax2 = fig.add_subplot(122)

        plt.sca(ax1)
        plt.plot(time, velocity_relative)
        plt.plot(time, velocity_global)

        plt.xlabel('Time [s]')
        plt.ylabel('Wind velocity [m/s]')

        plt.sca(ax2)
        plt.plot(time, direction_relative)
        plt.plot(time, direction_global)

        plt.xlabel('Time [s]')
        plt.ylabel('Wind direction [deg]')

    plt.show()