import pandas as pd

import numpy as numpy
import matplotlib.pyplot as plt

import os

from pathlib import Path

import argparse


if __name__ == '__main__':
    output_path = Path('../output_with_stormbird')

    all_output_files = os.listdir(output_path)

    sails_files = []
    for f in all_output_files:
        if 'sails' in f and '.csv' in f:
            sails_files.append(f)

    sails_files.sort()

    sails_df = pd.read_csv(output_path / Path(sails_files[0]))

    sobc1_files = []
    for f in all_output_files:
        if 'SOBC1' in f and '.csv' in f:
            sobc1_files.append(f)

    sobc1_files.sort()

    sobc1_df = pd.read_csv(output_path / Path(sobc1_files[0]))

    w_plot = 16
    h_plot = w_plot / 1.0
    fig = plt.figure(figsize=(w_plot, h_plot))

    ax1 = fig.add_subplot(211)
    ax2 = fig.add_subplot(212)

    ax3 = ax2.twinx()

    ax_list = [ax1, ax2]

    wave_roll_moment = (
        sobc1_df['firstOrderWaveForce.moment.roll'].to_numpy() + 
        sobc1_df['secondOrderWaveForce.moment.roll'].to_numpy()
    )

    plt.sca(ax1)
    plt.plot(sobc1_df['Time'], wave_roll_moment, label='Wave roll moment')
    plt.plot(sails_df['Time'], sails_df['moment_x'], label='Sails roll moment')
    plt.legend()
    

    plt.sca(ax2)
    #plt.plot(sobc1_df['Time'], sobc1_df['cgShipMotion.angularDisplacement.roll'])
    plt.plot(sobc1_df['Time'], sobc1_df['cgShipMotion.angularVelocity.roll'], label='Roll velocity')
    #plt.plot(sails_df['Time'], sails_df['rotation_x'])
    plt.sca(ax3)
    plt.plot(sails_df['Time'], sails_df['moment_x'], color='black', label='Sails roll moment')
    plt.legend()

    for ax in ax_list:
        plt.sca(ax)
        plt.xlim(0, 120)

    plt.show()