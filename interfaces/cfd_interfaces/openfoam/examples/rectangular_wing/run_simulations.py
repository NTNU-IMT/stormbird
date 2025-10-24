
import shutil
import subprocess
import pandas as pd
import matplotlib.pyplot as plt
import argparse

from stormbird_settings import StormbirdSettings
from openfoam_settings import OpenFOAMSettings, FolderPaths



if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument("--angle-of-attack", type=float, required=True, help="Angle of attacks to simulate")
    parser.add_argument("--use-ll-correction", action='store_true')

    args = parser.parse_args()

    result_data = []

    angle = args.angle_of_attack

    stormbird_settings = StormbirdSettings(
        angle_of_attack_deg=angle,
        use_ll_correction=args.use_ll_correction
    )

    folder_paths = FolderPaths(angle_of_attack_deg=angle)

    if folder_paths.run_folder.exists():
        shutil.rmtree(folder_paths.run_folder)

    shutil.copytree(folder_paths.base_folder, folder_paths.run_folder)

    stormbird_settings.write_actuator_line_setup_to_file(folder_paths.run_folder / 'system' /'stormbird_actuator_line.json')
    stormbird_settings.write_dimensions(folder_paths.run_folder / 'dimensions.txt')

    openfoam_settings = OpenFOAMSettings()

    openfoam_settings.write_to_file(folder_paths.run_folder / "simulation_settings.txt")
    openfoam_settings.set_as_environmental_variables()

    subprocess.run(['bash run.sh'], cwd=folder_paths.run_folder, shell=True)

    forces_df = pd.read_csv(folder_paths.run_folder / 'postProcessing' / 'stormbird_forces.csv')

    force_x = forces_df['force_0.x'].to_numpy() / stormbird_settings.force_factor
    force_y = forces_df['force_0.y'].to_numpy() / stormbird_settings.force_factor

    plt.plot(forces_df['time'], force_x, label="x")
    plt.plot(forces_df['time'], force_y, label="y")

    print('Last force, x', force_x[-1])
    print('Last force, y', force_y[-1])

    plt.legend()

    plt.savefig(folder_paths.run_folder / "force_plot.png", dpi=300, bbox_inches='tight')
