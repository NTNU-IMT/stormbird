'''
Script that simulates a heaving wing with both dynamic and quasi-static lifting line models. The 
result are compared against each other and against a theoretical (simplified) model.
'''

import json
from pathlib import Path

import numpy as np
import scipy.interpolate as interpolate
import matplotlib.pyplot as plt

from pystormbird.lifting_line import Simulation
from pystormbird import Vec3

import argparse

def get_motion_functions(*, amplitude: float, radial_frequency: float):
    '''
    Create closures for the motion as a function of time, based on the amplitude and radial frequency.
    '''
    def position(t: float):
        return amplitude * np.sin(radial_frequency * t)

    def velocity(t: float):
        return amplitude * radial_frequency * np.cos(radial_frequency * t)

    return position, velocity

def theodorsen_lift_reduction_data(reduced_frequency: float) -> float:
    ''' 
    Reduction of the lift due to dynamic effects according to Theodorsen's function, as presented
    in: "Bøckmann, E., 2015, "Wave Propulsion of Ships", page 28, Figure 3.3
    '''

    x_data = np.array([
        0.0000000000000000, 0.10160218835482548, 0.21101992966002325, 0.3243454474404064, 0.45720984759671746, 0.6213364595545132,
        0.8401719421649079, 1.0668229777256741, 1.3325517780382963, 1.6060961313012891, 2.0007815552950357
    ])

    y_data = np.array([
        0.9999999999999999, 0.8254545454545454, 0.7185454545454544, 0.6552727272727272, 0.6094545454545454, 0.5745454545454544, 
        0.5516363636363635, 0.5363636363636362, 0.5254545454545453, 0.5199999999999998, 0.5134545454545453
    ])

    spl = interpolate.splrep(x_data, y_data)

    return interpolate.splev(reduced_frequency, spl)

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Run simulation of a heaving wing")
    parser.add_argument("-rf", "--reduced-frequency", type=float, default = 0.3, help="Reduced frequency")
    parser.add_argument("--amplitude-factor", type=float, default = 0.1, help="Amplitude relative to chord length of heaving motion")
    parser.add_argument("--write-wake-files", action="store_true", help="Write wake files")

    args = parser.parse_args()

    default_colors = plt.rcParams['axes.prop_cycle'].by_key()['color']
    
    if args.write_wake_files:
        wake_files_folder_path  = Path("wake_files_output")

        if not wake_files_folder_path.exists():
            raise FileNotFoundError(
                f"Folder {wake_files_folder_path} does not exist. Create it to allow storage of wake files."
            )
    else:
        wake_files_folder_path = ''

    reduced_frequency = args.reduced_frequency

    velocity = 8.0
    chord_length = 1.0 # Deliberately chosen to be small, as the dynamic effects are easier to compare against theory when the 3D effects are small
    span = 32.0
    nr_sections = 64
    density = 1.225

    aspect_ratio = span / chord_length

    amplitude = args.amplitude_factor * chord_length

    radial_frequency = reduced_frequency * velocity / (0.5 * chord_length)
    frequency = radial_frequency / (2.0 * np.pi)
    period = 1.0 / frequency

    position_func, velocity_func = get_motion_functions(
        amplitude = amplitude, radial_frequency = radial_frequency
    )

    max_vertical_velocity = amplitude * radial_frequency

    max_angle_of_attack = np.arctan(max_vertical_velocity / velocity)

    max_cl_theory = 2.0 * np.pi * max_angle_of_attack / (1 + 2/aspect_ratio)

    force_factor = 0.5 * chord_length * span * density * velocity**2

    wings = [
        {
            "section_points": [
                {"x": 0.0, "y": 0.0, "z": -span/2.0},
                {"x": 0.0, "y": 0.0, "z": span/2.0}
            ],
            "chord_vectors": [
                {"x": chord_length, "y": 0.0, "z": 0.0},
                {"x": chord_length, "y": 0.0, "z": 0.0}
            ],
            "section_model": {
                "Foil": {}
            }
        }
    ]

    line_force_model_stat = {
        "wing_builders": wings,
        "nr_sections": nr_sections,
    }

    line_force_model_dyn = {
        "wing_builders": wings,
        "nr_sections": nr_sections,
        "ctrl_point_chord_factor": 0.0
    }

    solver_settings = {
        "max_iterations_per_time_step": 300,
        "damping_factor_start": 0.05,
        "damping_factor_end": 0.2,
    }

    dt = period / 128
    final_time = 5.0 * period

    relative_panel_length = dt * velocity / chord_length

    first_panel_relative_length = (dt / velocity) / chord_length

    sim_settings_list = [
        {
            "Dynamic": {
                "solver": solver_settings,
                "wake": {
                    "ratio_of_wake_affected_by_induced_velocities": 0.0,
                    "first_panel_relative_length": relative_panel_length,
                    "last_panel_relative_length": 20.0,
                    "wake_length": {
                        "NrPanels": 400
                    }
                }
            }
        },
        {
            "QuasiSteady": {
                "solver": solver_settings,
            }
        }
    ]

    
    line_force_model_list = [line_force_model_dyn, line_force_model_stat]

    label_list = ["Dynamic", "Quasi-steady"]
    color_list = [default_colors[0], default_colors[1]]


    w_plot = 14
    fig = plt.figure(figsize=(w_plot, w_plot / 3.0))

    max_cl = []

    for simulation_settings, line_force_model, label, color in zip(
        sim_settings_list,
        line_force_model_list,
        label_list,
        color_list
    ):
        print("Running ", label, "simulations:")

        setup = {
            "line_force_model": line_force_model,
            "simulation_mode": simulation_settings,
            "write_wake_data_to_file": args.write_wake_files,
            "wake_files_folder_path": str(wake_files_folder_path)
        }

        setup_string = json.dumps(setup)

        simulation = Simulation(
            setup_string = setup_string,
            initial_time_step = dt,
            wake_initial_velocity = Vec3(velocity, 0.0, 0.0), 
        )

        time = []
        lift = []
        drag = []

        t = 0.0

        '''
        Query the simulation struct for points where the freestream velocity is defined. This is 
        only done once in this case as the velocity is not dependent on the position of the wing.
        Also, because there is noe spatial variation in the velocity, the freestream velocity is
        the same for all points.
        '''
        freestream_velocity_points = simulation.get_freestream_velocity_points()

        freestream_velocity = []
        for point in freestream_velocity_points:
            freestream_velocity.append(Vec3(velocity, 0.0, 0.0))

        while t < final_time:
            print("Running sim at time = ", t)

            simulation.set_translation(Vec3(0.0, position_func(t), 0.0))

            result = simulation.do_step(
                time = t, 
                time_step = dt, 
                freestream_velocity = freestream_velocity,
            )

            forces = result.integrated_forces_sum()

            time.append(t)
            lift.append(forces.y / force_factor)
            drag.append(forces.x / force_factor)

            t += dt

        plt.plot(time, lift, label=label, color=color)

        max_cl.append(np.max(lift))

        print()

    theodorsen_reduction = theodorsen_lift_reduction_data(reduced_frequency)
    three_dim_reduction = 1 / (1 + 2/aspect_ratio)

    cl_quasi_steady_theory = -2 * np.pi * three_dim_reduction* velocity_func(np.array(time)) / velocity
    cl_theodorsen = cl_quasi_steady_theory * theodorsen_reduction

    plt.plot(
        time, 
        cl_theodorsen, 
        label="Simplified quasi-steady * Real(Theodorsen)", linestyle="--", color=color_list[0]
    )

    plt.plot(
        time, 
        cl_quasi_steady_theory, 
        label="Simplified quasi-steady", linestyle="--", color=color_list[1]
    )

    print("Theodorsen ratio", theodorsen_reduction)
    print("CL ratio", max_cl[0] / max_cl[1])

    plt.xlim(1.0 * period, 5 * period)
    plt.ylim(-1.1 * max_cl_theory, 1.1 * max_cl_theory)
    plt.xlabel("Time")
    plt.ylabel("Lift coefficient")
    plt.legend()

    plt.tight_layout()
    plt.savefig("heaving_wing.png", dpi=300, bbox_inches="tight")

    plt.show()