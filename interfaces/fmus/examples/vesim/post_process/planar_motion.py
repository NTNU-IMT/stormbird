import pandas as pd

import numpy as np
import matplotlib.pyplot as plt

import os

from pathlib import Path

import argparse

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description='Plot results.')
    parser.add_argument('--cases-to-plot', type=str, default='both', help='Cases to plot.')

    args = parser.parse_args()

    output_path_w_sb = Path('../output/output_with_stormbird')
    output_path_n_sb = Path('../output/output_no_stormbird')

    if args.cases_to_plot == 'both':
        output_path_list = [output_path_w_sb, output_path_n_sb]
        output_label = ['with sails', 'no sails']
    elif args.cases_to_plot == 'no-sails':
        output_path_list = [output_path_n_sb]
        output_label = ['no sails']

    w_plot = 16
    h_plot = w_plot / 1.5

    fig = plt.figure(figsize=(w_plot, h_plot))
    ax1 = fig.add_subplot(221)
    ax2 = fig.add_subplot(222)
    ax3 = fig.add_subplot(223)
    ax4 = fig.add_subplot(224)

    ax_list = [ax1, ax2, ax3, ax4]
    ax_titles = ['Ship path', 'Velocity', 'Course', 'Rudder angle']
    x_label = ['east [m]', 'time [s]', 'time [s]', 'time [s]']
    y_label = ['north [m]', 'velocity [m/s]', 'course [deg]', 'rudder angle [deg]']

    for index, output_path in enumerate(output_path_list):
        all_output_files = os.listdir(output_path)

        sobc_files = []
        rudder_files = []
        for f in all_output_files:
            if 'SOBC1' in f and '.csv' in f:
                sobc_files.append(f)
            if 'rudder' in f and not 'limiter' in f and '.csv' in f:
                rudder_files.append(f)

        sobc_files.sort()
        rudder_files.sort()

        sobc1_df = pd.read_csv(output_path / Path(sobc_files[0]))
        rudder_df = pd.read_csv(output_path / Path(rudder_files[0]))

        east_position = sobc1_df['cgShipMotion.nedDisplacement.east'].to_numpy()
        north_position = sobc1_df['cgShipMotion.nedDisplacement.north'].to_numpy()

        rudder_angle = rudder_df['output_angle'].to_numpy()
        course = sobc1_df['course'].to_numpy()
        drift_angle = sobc1_df['drift_angle'].to_numpy()
        yaw_angle = sobc1_df['cgShipMotion.angularDisplacement.yaw'].to_numpy()

        for i in range(len(yaw_angle)):
            if yaw_angle[i] > 180:
                yaw_angle[i] -= 360
            elif yaw_angle[i] < -180:
                yaw_angle[i] += 360

        time = sobc1_df['Time'].to_numpy()
        velocity_surge = sobc1_df['cgShipMotion.linearVelocity.surge'].to_numpy()
        velocity_sway  = sobc1_df['cgShipMotion.linearVelocity.sway'].to_numpy()

        velocity_mag = np.sqrt(velocity_surge**2 + velocity_sway**2)

        plt.sca(ax1)
        plt.plot(east_position, north_position, label=output_label[index])

        plt.xlim(-1.5*np.max(east_position), 1.5 * np.max(east_position))
        plt.ylim(0, np.max(north_position))

        plt.sca(ax2)
        plt.plot(time, velocity_mag, label=output_label[index])

        plt.ylim(0, 10)

        plt.sca(ax3)
        plt.plot(time, course, label=output_label[index])    
        plt.plot(time, yaw_angle, label='yaw angle')

        plt.sca(ax4)
        plt.plot(time, rudder_angle, label=output_label[index]) 

    for index, ax in enumerate(ax_list):
        plt.sca(ax)
        plt.title(ax_titles[index])
        plt.xlabel(x_label[index])
        plt.ylabel(y_label[index])
        plt.legend()

    plt.tight_layout()


    plt.show()