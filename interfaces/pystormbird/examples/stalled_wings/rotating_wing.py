'''
Script that several multi-element wings for a given wind condition, angle of attack and flap angles.

Note: currently all wings are assumed to operate with the same angle of attack and flap angles. 
However, this is planned to changed in the future.
'''

import json

import numpy as np
import matplotlib.pyplot as plt

from pystormbird.lifting_line import Simulation
from pystormbird import SpatialVector
import json

import argparse

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Run simulation of a multiple two-element wings")
    parser.add_argument("--wind-velocity",    type=float, default = 8.2,  help="Flap angle in degrees")
    parser.add_argument("--rotational-velocity", type=float, default = 10.0,  help="Rotational velocity in deg/s")
    parser.add_argument("--circulation-viscosity", type=float, default = 0.0,  help="Circulation viscosity")
    parser.add_argument("--gaussian-smoothing-length", type=float, default = 0.1,  help="Gaussian smoothing length")
    parser.add_argument("--damping-factor",  type=float, default = 0.01,  help="Damping factor")
    parser.add_argument("--nr-sections",  type=int, default = 32,  help="Nr sections per wing")

    parser.add_argument("--dynamic",          action="store_true", help="Use dynamic model")
    parser.add_argument("--write-wake-files", action="store_true", help="Write wake files")

    args = parser.parse_args()

    wind_velocity_vector = SpatialVector(args.wind_velocity, 0.0, 0.0)

    chord_length = 9.8
    span = 37.0

    density = 1.225

    start_height = 10.0

    x_positions = np.array([0.0])
    y_positions = np.array([0.0])

    chord_vector = SpatialVector(chord_length, 0.0, 0.0)

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
        "nr_sections": args.nr_sections,
        "density": density
    }

    solver_settings = {
        "damping_factor": args.damping_factor,
        "circulation_viscosity": args.circulation_viscosity,
    }

    if args.gaussian_smoothing_length > 0.0:
        solver_settings["gaussian_smoothing_length"] = args.gaussian_smoothing_length

    if args.dynamic:
        sim_settings = {
            "Dynamic": {
                "wake": {
                    "viscous_core_length_off_body": {
                        "Absolute": 0.25 * chord_length
                    }
                },
                "solver": solver_settings
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
                "solver": solver_settings
            }
        }

        setup = {
            "line_force_model": line_force_model,
            "simulation_mode": sim_settings
        }

    setup_string = json.dumps(setup)

    end_time = 360 / args.rotational_velocity
    dt = end_time / 256

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
    current_angle_deg = 0.0

    angles = []
    lift = []
    drag = []

    while current_time < end_time:
        current_angle_deg += args.rotational_velocity * dt

        print("Time: ", current_time)
        print("Current angle: ", current_angle_deg)
        print()

        simulation.set_local_wing_angles([np.radians(current_angle_deg)] * len(wings))

        result = simulation.do_step(
            time = current_time, 
            time_step = dt, 
            freestream_velocity = freestream_velocity
        )

        forces = result.integrated_forces_sum()

        angles.append(current_angle_deg)
        
        drag.append(forces.x / force_factor)
        lift.append(-forces.y / force_factor)

        current_time += dt


    plt.plot(angles, drag, label="Drag")
    plt.plot(angles, lift, label="Lift")
    plt.show()
    
