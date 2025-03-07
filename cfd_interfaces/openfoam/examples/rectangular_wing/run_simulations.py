
import shutil
import subprocess
import json

import argparse

from stormbird_settings import StormbirdSettings
from openfoam_settings import SimulationSettings, FolderPaths

from pystormbird import SimulationResult

import numpy as np
import matplotlib.pyplot as plt

if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument("--angle-of-attack", type=float, required=True, help="Angle of attacks to simulate")

    args = parser.parse_args()

    result_data = []

    angle = args.angle_of_attack

    stormbird_settings = StormbirdSettings(
        angle_of_attack_deg=angle
    )

    folder_paths = FolderPaths(angle_of_attack_deg=angle)

    if folder_paths.run_folder.exists():
        shutil.rmtree(folder_paths.run_folder)

    shutil.copytree(folder_paths.base_folder, folder_paths.run_folder)

    stormbird_settings.write_actuator_line_setup_to_file(folder_paths.run_folder / 'actuator_line.json')
    stormbird_settings.write_dimensions(folder_paths.run_folder / 'dimensions.txt')

    simulation_settings = SimulationSettings()

    simulation_settings.write_to_file(folder_paths.run_folder / "simulation_settings.txt")
    simulation_settings.set_as_environmental_variables()

    subprocess.run(['bash run.sh'], cwd=folder_paths.run_folder, shell=True)

    result_history = SimulationResult.result_history_from_file(
        str(folder_paths.run_folder / "actuator_line_results.json")
    )

    n_time_steps = len(result_history)

    x_forces = np.zeros(n_time_steps)
    y_forces = np.zeros(n_time_steps)

    for i in range(n_time_steps):
        x_forces[i] = result_history[i].integrated_forces[0].total.x
        y_forces[i] = result_history[i].integrated_forces[0].total.y

    plt.plot(x_forces, label="x")
    plt.plot(y_forces, label="y")

    plt.legend()
    
    plt.savefig(folder_paths.run_folder / "force_plot.png", dpi=300, bbox_inches='tight')