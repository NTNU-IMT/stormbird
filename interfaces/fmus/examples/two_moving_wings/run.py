
import subprocess

import time
from pathlib import Path

import os

import argparse

from simulation_builder import LiftingLineSimulation

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

    args = parser.parse_args()

    sim_setup = LiftingLineSimulation()

    write_motion_data(
        period = args.end_time / 4.0,
        end_time = args.end_time,
        time_step = 0.1,
        file_path = 'motion.csv'
    )

    sim_setup.write_fmu_parameters(Path('stormbird_parameters.json'))

    output_path = Path('output')

    delete_all_result_files_in_folder(output_path)

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
