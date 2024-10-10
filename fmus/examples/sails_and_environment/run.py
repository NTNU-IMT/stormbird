
import subprocess

import time
from pathlib import Path

import os

import argparse

from stormbird_setup import make_stormbird_setup_file, make_sail_controller_setup_file

def delete_old_output_files(output_path: Path):
    current_output_files = os.listdir(output_path)

    for file in current_output_files:
        file_path = output_path / file
        try:
            if file_path.is_file() and file != '.gitignore':
                file_path.unlink()
        except Exception as e:
            print(e)

def delete_old_wake_files():
    wake_path = Path('wake_files')
    current_wake_files = os.listdir(wake_path)

    for file in current_wake_files:
        file_path = wake_path / file
        try:
            if file_path.is_file() and file != '.gitignore':
                file_path.unlink()
        except Exception as e:
            print(e)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description='Run simulation.')
    parser.add_argument('--end-time', type=float, default=5.0, help='End time of simulation.')

    make_stormbird_setup_file()
    make_sail_controller_setup_file()

    args = parser.parse_args()

    output_path = Path('output')

    delete_old_output_files(output_path)
    delete_old_wake_files()

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