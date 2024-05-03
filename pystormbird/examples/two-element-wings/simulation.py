'''
Script that several multi-element wings for a given wind condition, angle of attack and flap angles.

Note: currently all wings are assumed to operate with the same angle of attack and flap angles. 
However, this is planned to changed in the future.
'''

import json

import numpy as np
import matplotlib.pyplot as plt

from pystormbird.lifting_line import Simulation
from pystormbird import Vec3
import json

from foil_model import get_foil_dict

import argparse

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Run simulation of a multiple two-element wings")
    parser.add_argument("--flap-angle", type=float, default = 5.0, help="Flap angle in degrees")
    parser.add_argument("--angle-of-attack", type=float, default = 0.0, help="Angle of attack in degrees")
    parser.add_argument("--wind-velocity", type=float, default = 8.0, help="Wind velocity in m/s")
    parser.add_argument("--wind-direction", type=float, default = 45.0, help="Wind direction in degrees")
    parser.add_argument("--dynamic", action="store_true", help="Use dynamic model")
    parser.add_argument("--write-wake-files", action="store_true", help="Write wake files")

    args = parser.parse_args()

    velocity = args.wind_velocity

    chord_length = 9.8
    span = 37.0

    nr_sections = 16
    density = 1.225

    start_height = 10.0

    x_positions = np.array([-50.0, 50.0])
    y_positions = np.array([-10, 10.0])

    foil_dict = get_foil_dict(flap_angle=np.radians(args.flap_angle))

    wings = []

    chord_angle = np.radians(
        args.wind_direction - args.angle_of_attack
    )

    chord_vector = Vec3(
        chord_length * np.cos(chord_angle),
        chord_length * np.sin(chord_angle),
        0.0
    )

    for i in range(len(x_positions)):
        wings.append(
            {
                "section_points": [
                    {"x": x_positions[i], "y": y_positions[i], "z": start_height},
                    {"x": x_positions[i], "y": y_positions[i], "z": start_height + span}
                ],
                "chord_vectors": [
                    {"x": chord_vector.x, "y": chord_vector.y, "z": chord_vector.z},
                    {"x": chord_vector.x, "y": chord_vector.y, "z": chord_vector.z}
                ],
                "section_model": {
                    "VaryingFoil": foil_dict
                }
            }
        )

    force_factor = 0.5 * chord_length * span * density * velocity**2 * len(wings)

    line_force_model = {
        "wing_builders": wings,
        "nr_sections": nr_sections,
        "density": density
    }

    if args.dynamic:
        '''
        Default settings for a dynamic simulation, except for one option: the viscous core length 
        off body.

        This setting is used to stabilize the shape integration used when the lift coefficient is 
        high. It typically do not affect the forces significantly.
        '''
        sim_settings = {
            "Dynamic": {
                "wake": {
                    "viscous_core_length_off_body": {
                        "Absolute": 0.25 * chord_length
                    }
                }
            }
        }

        setup = {
            "line_force_model": line_force_model,
            "simulation_mode": sim_settings,
            "write_wake_data_to_file": args.write_wake_files,
            "wake_files_folder_path": "wake_files_output"
        }

    else:
        sim_settings = {
            "QuasiSteady": {}
        }

        setup = {
            "line_force_model": line_force_model,
            "simulation_mode": sim_settings
        }

    setup_string = json.dumps(setup)

    simulation = Simulation.new_from_string(setup_string)

    if args.dynamic:
        end_time = 20 * chord_length / velocity
        dt = end_time / 128
    else:
        end_time = 1.0
        dt = 1.0
    
    current_time = 0.0

    '''
    Note: the freestream velocity functionality in Python is currently limited to constant velocity.

    However, more advanced options exist the Rust library. An interface to this is on its way...
    '''
    freestream = Vec3(
        velocity * np.cos(np.radians(args.wind_direction)), 
        velocity * np.sin(np.radians(args.wind_direction)), 
        0.0
    )

    while current_time < end_time:
        result = simulation.do_step(
            time = current_time, 
            time_step = dt, 
            freestream_velocity = freestream
        )

        current_time += dt

    forces = result.integrated_forces_sum()

    print("Thrust factor:     ", -forces.x / force_factor)
    print("Side force factor: ", forces.y / force_factor)
    
