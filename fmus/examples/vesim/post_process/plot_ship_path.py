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

    output_path_list = [output_path_w_sb, output_path_n_sb]

    w_plot = 18
    h_plot = w_plot / 3.0

    fig = plt.figure(figsize=(w_plot, h_plot))
    ax1 = fig.add_subplot(131)
    ax2 = fig.add_subplot(132)
    ax3 = fig.add_subplot(133)

    for output_path in output_path_list:
        all_output_files = os.listdir(output_path)

        sobc_files = []
        for f in all_output_files:
            if 'SOBC1' in f and '.csv' in f:
                sobc_files.append(f)

        sobc_files.sort()

        sobc1_df = pd.read_csv(output_path / Path(sobc_files[0]))

        x = sobc1_df['cgShipMotion.nedDisplacement.north'].to_numpy()
        y = sobc1_df['cgShipMotion.nedDisplacement.east'].to_numpy()

        course = sobc1_df['course'].to_numpy()
        yaw    = sobc1_df['cgShipMotion.angularDisplacement.yaw'].to_numpy()

        time = sobc1_df['Time'].to_numpy()
        velocity_surge = sobc1_df['cgShipMotion.linearVelocity.surge'].to_numpy()
        velocity_sway  = sobc1_df['cgShipMotion.linearVelocity.sway'].to_numpy()

        velocity_mag = np.sqrt(velocity_surge**2 + velocity_sway**2)

        plt.sca(ax1)
        plt.plot(y, x)
        plt.xlabel('north [m]')
        plt.ylabel('east [m]')

        plt.xlim(-0.25*np.max(x), 0.25 * np.max(x))
        plt.ylim(0, np.max(x))

        plt.sca(ax2)
        plt.plot(time, velocity_mag)
        plt.xlabel('time [s]')
        plt.ylabel('velocity [m/s]')

        plt.ylim(0, 10)

        plt.sca(ax3)
        plt.plot(time, course)
        plt.plot(time, yaw)
        plt.xlabel('time [s]')
        plt.ylabel('course [deg]')

    plt.sca(ax1)
    plt.title('Ship path')

    plt.sca(ax2)
    plt.title('Velocity history')

    plt.tight_layout()


    plt.show()