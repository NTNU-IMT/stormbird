import pandas as pd

import numpy as numpy
import matplotlib.pyplot as plt

import os

from pathlib import Path

import argparse

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description='Plot results.')
    parser.add_argument('--with-stormbird', action='store_true', help='Run with stormbird.')

    args = parser.parse_args()

    output_path_w_sb = Path('../output/output_with_stormbird')
    output_path_n_sb = Path('../output/output_no_stormbird')

    output_path_list = [output_path_w_sb, output_path_n_sb]
    output_names = ['with stormbird', 'without stormbird']

    w_plot = 16
    h_plot = w_plot / 3.0
    fig = plt.figure(figsize=(w_plot, h_plot))

    ax1 = fig.add_subplot(131)
    ax2 = fig.add_subplot(132)
    ax3 = fig.add_subplot(133)

    for output_path, name in zip(output_path_list, output_names):
        all_output_files = os.listdir(output_path)

        print(all_output_files)

        sobc_files = []
        for f in all_output_files:
            if 'SOBC1' in f and '.csv' in f:
                sobc_files.append(f)

        sobc_files.sort()

        sobc1_df = pd.read_csv(output_path / Path(sobc_files[0]))

        time = sobc1_df['Time'].to_numpy()

        roll  = sobc1_df['cgShipMotion.angularDisplacement.roll'].to_numpy()
        pitch = sobc1_df['cgShipMotion.angularDisplacement.pitch'].to_numpy()
        heave = sobc1_df['cgShipMotion.nedDisplacement.down'].to_numpy()
        
        plt.sca(ax1)
        plt.plot(time, roll, label=name)
        plt.ylabel('roll [deg]')

        plt.sca(ax2)
        plt.plot(time, pitch, label=name)
        plt.xlabel('time [s]')
        plt.ylabel('pitch [deg]')

        plt.sca(ax3)
        plt.plot(time, heave, label=name)
        plt.ylabel('heave [m]')

    
    plt.legend(loc='upper right')
    plt.tight_layout()
        

    plt.show()