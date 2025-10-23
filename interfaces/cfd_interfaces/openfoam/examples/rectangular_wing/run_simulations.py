
import shutil
import subprocess
import pandas as pd
import matplotlib.pyplot as plt
import argparse

from stormbird_settings import StormbirdSettings
from openfoam_settings import SimulationSettings, FolderPaths



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

    stormbird_settings.write_actuator_line_setup_to_file(folder_paths.run_folder / 'system' /'stormbird_actuator_line.json')
    stormbird_settings.write_dimensions(folder_paths.run_folder / 'dimensions.txt')

    simulation_settings = SimulationSettings()

    simulation_settings.write_to_file(folder_paths.run_folder / "simulation_settings.txt")
    simulation_settings.set_as_environmental_variables()

    subprocess.run(['bash run.sh'], cwd=folder_paths.run_folder, shell=True)

    forces_df = pd.read_csv(folder_paths.run_folder / 'postProcessing' / 'stormbird_forces.csv')

    plt.plot(forces_df['time'], forces_df['force_0.x'], label="x")
    plt.plot(forces_df['time'], forces_df['force_0.y'], label="y")

    print('Last force, x', forces_df['force_0.x'].iloc[-1])
    print('Last force, y', forces_df['force_0.y'].iloc[-1])

    plt.legend()

    plt.savefig(folder_paths.run_folder / "force_plot.png", dpi=300, bbox_inches='tight')
