
import subprocess

import time
from pathlib import Path

import os
import shutil

import argparse

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
    parser.add_argument('--end-time', type=float, default=10.0, help='End time of simulation.')

    args = parser.parse_args()

    cosim_path = Path.home() / Path("Software/open_simulation_platform/cosim-v0.7.1-win64/bin/cosim.exe")
    
    output_path = Path('output')

    delete_old_output_files(output_path)
    delete_old_wake_files()

    start_time = time.time()
    subprocess.run(
        [
            str(cosim_path), 
            'run', '.', 
            '--end-time', str(args.end_time), 
            '--output-dir', str(output_path)
        ]
    )
    end_time = time.time()

    print('Total time: {}s'.format(end_time - start_time))