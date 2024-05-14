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

import argparse

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Run simulation of a multiple two-element wings")
    parser.add_argument("--wind-velocity",    type=float, default = 8.2,  help="Flap angle in degrees")
    parser.add_argument("--angle-of-attack",  type=float, default = 75.0,  help="Angle of attack in degrees")
    parser.add_argument("--circulation-viscosity", type=float, default = 0.0,  help="Circulation viscosity")
    parser.add_argument("--damping-factor",  type=float, default = 0.01,  help="Damping factor")
    parser.add_argument("--dynamic",          action="store_true", help="Use dynamic model")
    parser.add_argument("--write-wake-files", action="store_true", help="Write wake files")

    args = parser.parse_args()

    wind_velocity_vector = Vec3(args.wind_velocity, 0.0, 0.0)

    chord_length = 9.8
    span = 37.0

    nr_sections = 32
    density = 1.225

    start_height = 10.0

    x_positions = np.array([0.0])
    y_positions = np.array([0.0])

    chord_angle = np.radians(args.angle_of_attack)

    chord_vector = Vec3(
        chord_length * np.cos(chord_angle),
        chord_length * np.sin(chord_angle),
        0.0
    )

    wings = []
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
                    "Foil": {}
                }
            }
        )

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
                },
                "solver": {
                    "circulation_viscosity": args.circulation_viscosity
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
            "QuasiSteady": {
                "solver": {
                    "damping_factor": args.damping_factor,
                    "circulation_viscosity": args.circulation_viscosity
                }
            }
        }

        setup = {
            "line_force_model": line_force_model,
            "simulation_mode": sim_settings
        }

    setup_string = json.dumps(setup)

    if args.dynamic:
        end_time = 20 * chord_length / args.wind_velocity
        dt = end_time / 128
    else:
        end_time = 1.0
        dt = 1.0

    simulation = Simulation(
        setup_string = setup_string,
        initial_time_step = dt,
        wake_initial_velocity = wind_velocity_vector
    )

    '''
    Create the freestream velocity vector, which depends on the z coordinate of the points. However, 
    it does not change as a function of time, so it is created once and used for all time steps.
    '''

    freestream_velocity_points = simulation.get_freestream_velocity_points()

    freestream_velocity = []
    for point in freestream_velocity_points:
        freestream_velocity.append(
            wind_velocity_vector
        )

    force_factor = 0.5 * chord_length * span * density * args.wind_velocity**2 * len(wings)
    
    current_time = 0.0

    while current_time < end_time:
        print("Time: ", current_time)
        result = simulation.do_step(
            time = current_time, 
            time_step = dt, 
            freestream_velocity = freestream_velocity
        )

        current_time += dt

    forces = result.integrated_forces_sum()

    print("Thrust factor:     ", -forces.x / force_factor)
    print("Side force factor: ", forces.y / force_factor)

    ctrl_points = result.ctrl_points

    z_coords = []
    for point in ctrl_points:
        z_coords.append(point.z)

    for i in range(len(wings)):
        plt.plot(
            z_coords[i * nr_sections: (i + 1) * nr_sections], 
            result.circulation_strength[i * nr_sections: (i + 1) * nr_sections]
        )

    plt.show()
    
