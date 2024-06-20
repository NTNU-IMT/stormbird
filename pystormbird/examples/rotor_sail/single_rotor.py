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
    
    parser.add_argument("--wind-velocity",             type=float, default = 8.2,  help="Flap angle in degrees")
    parser.add_argument("--start-height",              type=float, default = 10.0, help="Start height")
    parser.add_argument("--spin-ratio",                type=float, default = 2.0,  help="Spin ratio")
    parser.add_argument("--circulation-viscosity",     type=float, default = 0.0,  help="Circulation viscosity")
    parser.add_argument("--gaussian-smoothing-length", type=float, default = 0.0,  help="Gaussian smoothing length")
    parser.add_argument("--damping-factor",            type=float, default = 0.01, help="Damping factor")
    parser.add_argument("--max-induced-velocity-ratio", type=float, default = 1.0,  help="Max induced velocity ratio")
    parser.add_argument("--nr-sections",               type=int, default = 32,  help="Nr sections per wing")

    parser.add_argument("--dynamic",          action="store_true", help="Use dynamic model")
    parser.add_argument("--write-wake-files", action="store_true", help="Write wake files")

    args = parser.parse_args()

    wind_velocity_vector = Vec3(args.wind_velocity, 0.0, 0.0)

    diameter = 5.0
    span = 35.0

    density = 1.225

    x_positions = np.array([0.0])
    y_positions = np.array([0.0])

    chord_vector = Vec3(diameter, 0.0, 0.0)

    circumference = np.pi * diameter
    tangential_velocity = args.wind_velocity * args.spin_ratio
            
    revolutions_per_second = -tangential_velocity / circumference

    rotors = []
    for i in range(len(x_positions)):
        rotors.append(
            {
                "section_points": [
                    {"x": x_positions[i], "y": y_positions[i], "z": args.start_height},
                    {"x": x_positions[i], "y": y_positions[i], "z": args.start_height + span}
                ],
                "chord_vectors": [
                    {"x": chord_vector.x, "y": chord_vector.y, "z": chord_vector.z},
                    {"x": chord_vector.x, "y": chord_vector.y, "z": chord_vector.z}
                ],
                "section_model": {
                    "RotatingCylinder": {
                        "revolutions_per_second": revolutions_per_second
                    }
                }
            }
        )

    line_force_model = {
        "wing_builders": rotors,
        "nr_sections": args.nr_sections,
        "density": density
    }

    solver_settings = {
        "damping_factor": args.damping_factor,
        "circulation_viscosity": args.circulation_viscosity,
    }

    if args.gaussian_smoothing_length > 0.0:
        solver_settings["gaussian_smoothing_length"] = args.gaussian_smoothing_length

    wake_settings = {
        "symmetry_condition": "Z",
        "induced_velocity_corrections": {
            "max_magnitude_ratio": args.max_induced_velocity_ratio
        }
    }

    if args.dynamic:
        solver_settings['max_iterations_per_time_step'] = 3

        wake_settings["first_panel_relative_length"] = 0.5

        wake_settings["viscous_core_length_off_body"] = {
            "Absolute": 0.5 * diameter
        }

        wake_settings["ratio_of_wake_affected_by_induced_velocities"] = 0.1

        sim_settings = {
            "Dynamic": {
                "wake": wake_settings,
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
                "wake": wake_settings,
                "solver": solver_settings
            }
        }

        setup = {
            "line_force_model": line_force_model,
            "simulation_mode": sim_settings
        }

    setup_string = json.dumps(setup)

    if args.dynamic:
        end_time = 80 * diameter / args.wind_velocity
        dt = end_time / 256
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

    force_factor = 0.5 * diameter * span * density * args.wind_velocity**2 * len(rotors)
    
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

    print("Drag:", forces.x / force_factor)
    print("Lift: ", forces.y / force_factor)

    ctrl_points = result.ctrl_points

    z_coords = []
    for point in ctrl_points:
        z_coords.append(point.z)

    for i in range(len(rotors)):
        indices = slice(i * args.nr_sections, (i + 1) * args.nr_sections)
        
        plt.plot(z_coords[indices], -np.array(result.circulation_strength[indices]))

    plt.ylim(0.0, None)

    plt.show()
    
