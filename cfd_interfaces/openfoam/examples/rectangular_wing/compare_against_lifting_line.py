import matplotlib.pyplot as plt
import numpy as np

import json

from pystormbird.lifting_line import Simulation
from pystormbird import Vec3

from stormbird_settings import StormbirdSettings
from folder_paths import FolderPaths

def run_lifting_line_simulation(stormbird_settings: StormbirdSettings):
    line_force_model = stormbird_settings.get_line_force_model_dict()

    gaussian_smoothing_settings = {
        "length_factor": 0.02,
        "end_corrections": [(True, True)]
    }

    line_force_model["smoothing_settings"] = {
        "gaussian": gaussian_smoothing_settings
    }

    solver = {
        "damping_factor_start": 0.01,
        "damping_factor_end": 0.1,
        "max_iterations_per_time_step": 10
    }

    lifting_line_setup = {
        "line_force_model": line_force_model,
        "simulation_mode": {
            "QuasiSteady": {
                "solver": solver
            }
        }
    }

    end_time = 20.0
    dt = 0.1

    sim = Simulation(
        setup_string = json.dumps(lifting_line_setup),
        initial_time_step = dt,
        wake_initial_velocity = Vec3(stormbird_settings.velocity, 0.0, 0.0)
    )

    freestream_velocity_points = sim.get_freestream_velocity_points()

    freestream_velocity = Vec3(stormbird_settings.velocity, 0.0, 0.0)

    freestream_velocity_list = []
    for _ in freestream_velocity_points:
        freestream_velocity_list.append(
            freestream_velocity
        )

    result_history = []
    current_time = 0
    while current_time < end_time:
        result = sim.do_step(
            time = current_time,
            time_step = dt,
            freestream_velocity = freestream_velocity_list
        )

        current_time += dt

        result_history.append(result)

    return result_history

if __name__ == "__main__":
    with open("results.json", "r") as f:
        actuator_results = json.load(f)

    angles_of_attack_al = np.array([result["angle_of_attack"] for result in actuator_results])
    cl_al = np.array([result["cl"] for result in actuator_results])
    cd_al = np.array([result["cd"] for result in actuator_results])

    angles_of_attack_ll = np.arange(0.0, 31, 1.0)
    cd_ll = []
    cl_ll = []

    for angle in angles_of_attack_ll:
        stormbird_settings = StormbirdSettings(
            angle_of_attack_deg=angle
        )
        result_history = run_lifting_line_simulation(stormbird_settings)

        forces = result_history[-1].integrated_forces[0].total

        cd_ll.append(forces.x / stormbird_settings.force_factor)
        cl_ll.append(forces.y / stormbird_settings.force_factor)

    w_plot = 16
    fig = plt.figure(figsize=(w_plot, w_plot/2.35))
    ax1 = fig.add_subplot(121)
    ax2 = fig.add_subplot(122)

    plt.sca(ax1)
    if len(angles_of_attack_al) == 1:
        plt.scatter(angles_of_attack_al, cd_al, label="Actuator line")
    else:
        plt.plot(angles_of_attack_al, cd_al, '-o', label="Actuator line")

    plt.plot(angles_of_attack_ll, cd_ll, label="Lifting line")

    plt.xlabel("Angle of attack [deg]")
    plt.ylabel("Cd")

    plt.ylim(0.0, None)

    plt.sca(ax2)
    if len(angles_of_attack_al) == 1:
        plt.scatter(angles_of_attack_al, cl_al, label="Actuator line")
    else:
        plt.plot(angles_of_attack_al, cl_al, '-o', label="Actuator line")

    plt.plot(angles_of_attack_ll, cl_ll, label="Lifting line")

    plt.xlabel("Angle of attack [deg]")
    plt.ylabel("Cl")

    plt.legend()

    plt.ylim(0.0, None)

    plt.show()



