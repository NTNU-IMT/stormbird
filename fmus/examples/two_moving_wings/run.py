
import subprocess

import time
from pathlib import Path

import os

import argparse

from stormbird_setup import LiftingLineSimulation, SailController, WindEnvironment
from motion_data import write_motion_data

def delete_all_result_files_in_folder(folder_path: Path):
    '''
    Function used to clean up result files in a folder.
    '''

    file_endings_to_delete = ['.csv', '.yaml', '.vtp']

    files_in_folder = os.listdir(folder_path)

    for file in files_in_folder:
        file_path = output_path / file
        try:
            if file_path.is_file() and file_path.suffix in file_endings_to_delete and file != '.gitignore':
                file_path.unlink()
        except Exception as e:
            print(e)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description='Run simulation.')
    parser.add_argument('--end-time', type=float, default=100.0, help='End time of simulation.')
    parser.add_argument("--write-wake-files", action="store_true", help="Write wake files.")

    args = parser.parse_args()

    write_motion_data(
        period = args.end_time / 2.0,
        end_time = args.end_time,
        time_step = 0.1,
        file_path = 'motion.csv'
    )

    lifting_line_simulation = LiftingLineSimulation(
        write_wake_files=args.write_wake_files
    )
    lifting_line_simulation.to_json_file('lifting_line_setup.json')

    controller = SailController()
    controller.to_json_file('sail_controller_setup.json')

    wind_environment = WindEnvironment()
    wind_environment.to_json_file('wind_environment_setup.json')

    

    output_path = Path('output')
    wake_files_path = Path('wake_files')

    delete_all_result_files_in_folder(output_path)
    delete_all_result_files_in_folder(wake_files_path)

    start_time = time.time()
    subprocess.run(
        [
            'cosim',
            'run', '.',
            '--end-time', str(args.end_time),
            '--output-dir', str(output_path),
        ],
    )
    end_time = time.time()

    simulation_time = end_time - start_time

    print('Total time: {}s'.format(simulation_time))
    print("Real time / simulation time: {}s".format(args.end_time / simulation_time))
