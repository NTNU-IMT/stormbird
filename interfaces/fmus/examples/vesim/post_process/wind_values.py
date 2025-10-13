import pandas as pd

import numpy as np
import matplotlib.pyplot as plt

import os

from pathlib import Path

'''
Script that plots wind values from the output files of the simulation. The purpose is to check that 
the input is as expected and that the conversion between the vessel FMU from ShipX and Stormbird is
done correctly.
'''

if __name__ == '__main__':
    output_path = Path('../output/output_with_stormbird')

    w_plot = 16
    h_plot = w_plot / 2.35

    fig = plt.figure(figsize=(w_plot, h_plot))
    ax1 = fig.add_subplot(121)
    ax2 = fig.add_subplot(122)

   
    all_output_files = os.listdir(output_path)

    sobc_files = []
    sail_files = []
    velocity_input_files = []
    for f in all_output_files:
        if 'SOBC1' in f and '.csv' in f:
            sobc_files.append(f)
        if 'sail' in f and '.csv' in f:
            sail_files.append(f)
        if 'velocity_input' in f and '.csv' in f:
            velocity_input_files.append(f)

    sobc_files.sort()
    sail_files.sort()
    velocity_input_files.sort()

    sobc1_df = pd.read_csv(output_path / Path(sobc_files[0]))
    sail_df = pd.read_csv(output_path / Path(sail_files[0]))
    velocity_input_df = pd.read_csv(output_path / Path(velocity_input_files[0]))

    time = sobc1_df['Time'].to_numpy()

    global_wind_vel = sobc1_df['global_wind_vel'].to_numpy()
    global_wind_dir = sobc1_df['global_wind_dir'].to_numpy()

    constant_velocity_x = velocity_input_df['constant_velocity_x'].to_numpy()
    constant_velocity_y = velocity_input_df['constant_velocity_y'].to_numpy()


    reference_wind_velocity_x = sail_df['reference_wind_velocity_x'].to_numpy()
    reference_wind_velocity_y = sail_df['reference_wind_velocity_y'].to_numpy()

    print(reference_wind_velocity_x)
    print(reference_wind_velocity_y)

    reference_wind_direction = np.degrees(np.arctan2(reference_wind_velocity_y, reference_wind_velocity_x))

    constant_velocity_mag = np.sqrt(constant_velocity_x**2 + constant_velocity_y**2)
    reference_wind_velocity_mag = np.sqrt(reference_wind_velocity_x**2 + reference_wind_velocity_y**2)

    plt.sca(ax1)
    plt.plot(time, global_wind_vel, label='Global wind velocity')
    plt.plot(time, constant_velocity_mag, label='Constant velocity magnitude')
    plt.plot(time, reference_wind_velocity_mag, label='Reference wind velocity magnitude')

    plt.xlabel('Time [s]')
    plt.ylabel('Velocity [m/s]')

    plt.sca(ax2)
    plt.plot(time, global_wind_dir, label='Global wind direction')
    plt.plot(time, reference_wind_direction, label='Reference wind direction')
    

    plt.xlabel('Time [s]')
    plt.ylabel('Direction [deg]')


    plt.show()