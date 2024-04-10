
import subprocess

import time
from pathlib import Path

import os

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
    wake_path = Path('output/wake_files')
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
    parser.add_argument('--end-time', type=float, default=600.0, help='End time of simulation.')
    parser.add_argument('--with-stormbird', action='store_true', help='Run with stormbird.')
    
    args = parser.parse_args()

    cosim_executable_path = Path("C:/Program Files/Open Simulation Platform/cosim-v0.7.1-win64/bin/cosim.exe") # Must be updated based on your installation
    
    if args.with_stormbird:
        output_path = Path('output/output_with_stormbird')
    else:
        output_path = Path('output/output_no_stormbird')

    delete_old_output_files(output_path)
    delete_old_wake_files()

    if args.with_stormbird:
        sim_name = 'osp_simulation_files/OspSystemStructure_with_stormbird.xml'
    else:
        sim_name = 'osp_simulation_files/OspSystemStructure_with_controllers.xml'

    start_time = time.time()
    subprocess.run(
        [
            str(cosim_executable_path), 
            'run', sim_name, 
            '--end-time', str(args.end_time), 
            '--output-dir', str(output_path)
        ],
        shell=True
    )
    end_time = time.time()

    print('Total time: {}s'.format(end_time - start_time))