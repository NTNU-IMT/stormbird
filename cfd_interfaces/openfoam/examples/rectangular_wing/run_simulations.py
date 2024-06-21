
import shutil
import subprocess
import json

import argparse

from stormbird_settings import StormbirdSettings
from folder_paths import FolderPaths

from pystormbird import SimulationResult

if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--angle-of-attacks", 
        type=float, nargs='+', required=True, help="List of angle of attacks to simulate"
    )

    args = parser.parse_args()

    result_data = []

    for angle in args.angle_of_attacks:
        print(f"Running simulation for angle of attack: {angle}")

        stormbird_settings = StormbirdSettings(
            angle_of_attack_deg=angle
        )

        folder_paths = FolderPaths(angle_of_attack_deg=angle)

        if folder_paths.run_folder.exists():
            shutil.rmtree(folder_paths.run_folder)

        shutil.copytree(folder_paths.base_folder, folder_paths.run_folder)

        stormbird_settings.write_actuator_line_setup_to_file(folder_paths.run_folder / 'actuator_line.json')
        stormbird_settings.write_dimensions(folder_paths.run_folder / 'dimensions.txt')

        subprocess.run(['bash run.sh'], cwd=folder_paths.run_folder, shell=True)

        result_history = SimulationResult.result_history_from_file(
            str(folder_paths.run_folder / "actuator_line_results.json")
        )

        forces = result_history[-1].integrated_forces[0].total

        result_data.append(
            {
                "angle_of_attack": angle,
                "cd": forces.x / stormbird_settings.force_factor,
                "cl": forces.y / stormbird_settings.force_factor,
            }
        )

        with open("results.json", "w") as f:
            json.dump(result_data, f, indent=4)