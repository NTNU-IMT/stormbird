import json
from pathlib import Path

import numpy as np
import matplotlib.pyplot as plt

from pystormbird.lifting_line import Simulation
from pystormbird import Vec3

import argparse

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Run simulation of a heaving wing")
    parser.add_argument("-rf", "--reduced-frequency", type=float, help="Reduced frequency")

    home = Path.home()

    wake_files_folder_path  = home / Path("wake_files")

    if not wake_files_folder_path.exists():
        raise FileNotFoundError(f"Folder {wake_files_folder_path} does not exist")

    args = parser.parse_args()

    reduced_frequency = args.reduced_frequency

    velocity = 8.0
    chord_length = 1.0
    span = 16.0
    nr_sections = 20

    amplitude = 0.1 * chord_length

    radial_frequency = reduced_frequency * velocity / (0.5 * chord_length)
    frequency = radial_frequency / (2.0 * np.pi)
    period = 1.0 / frequency

    force_factor = 0.5 * chord_length * span * velocity**2

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

    line_force_model = {
        "wing_builders": wings,
        "nr_sections": nr_sections,
    }

    sim_settings_list = [
        {
            "Dynamic": {}
        },
        {
            "QuasiSteady": {}
        }
    ]

    label_list = ["Dynamic", "QuasiSteady"]

    dt = period / 64
    final_time = 10.0 * period

    w_plot = 14
    fig = plt.figure(figsize=(w_plot, w_plot / 3.0))

    max_cl = []

    for simulation_settings, label in zip(sim_settings_list, label_list):
        setup = {
            "line_force_model": line_force_model,
            "simulation_mode": simulation_settings,
            "write_wake_data_to_file": True,
            "wake_files_folder_path": str(wake_files_folder_path)
        }

        setup_string = json.dumps(setup)

        simulation = Simulation.new_from_string(setup_string)

        time = []
        lift = []
        drag = []

        t = 0.0

        while t < final_time:
            print("Running sim at time = ", t)

            translation = Vec3(0.0, amplitude * np.sin(radial_frequency * t), 0.0)

            result = simulation.do_step(
                time = t, 
                time_step = dt, 
                freestream_velocity = Vec3(velocity, 0.0, 0.0),
                translation = translation
            )

            forces = result.integrated_forces_sum()

            time.append(t)
            lift.append(forces.y / force_factor)
            drag.append(forces.x / force_factor)

            t += dt

        plt.plot(time, lift, label=label)

        max_cl.append(np.max(lift))

    print("CL ratio", max_cl[0] / max_cl[1])

    plt.xlim(1.0 * period, 6 * period)
    plt.ylim(-0.5, 0.5)
    plt.xlabel("Time")
    plt.ylabel("Lift coefficient")
    plt.legend()

    plt.tight_layout()
    plt.savefig("heaving_wing.png", dpi=300, bbox_inches="tight")

    plt.show()