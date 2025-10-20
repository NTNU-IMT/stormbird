'''
Script that simulates a heaving wing with both dynamic and quasi-static lifting line models. The
result are compared against each other and against a theoretical (simplified) model.
'''

import time as time_func
import json
from pathlib import Path

import numpy as np
import scipy.interpolate as interpolate
import matplotlib.pyplot as plt

from stormbird_setup.direct_setup.spatial_vector import SpatialVector
from stormbird_setup.direct_setup.section_models import SectionModel, Foil
from stormbird_setup.direct_setup.line_force_model import LineForceModelBuilder, WingBuilder
from stormbird_setup.direct_setup.lifting_line.simulation_builder import SimulationBuilder, DynamicSettings, QuasiSteadySettings
from stormbird_setup.direct_setup.lifting_line.wake import DynamicWakeBuilder


from pystormbird.lifting_line import Simulation

import argparse

def get_motion_functions(*, amplitude: float, radial_frequency: float):
    '''
    Create closures for the motion as a function of time, based on the amplitude and radial frequency.
    '''
    def position(t: float) -> float:
        return amplitude * np.sin(radial_frequency * t)

    def velocity(t: float) -> float:
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
    nr_sections = 32
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

    dt = 0.25 * chord_length / velocity
    final_time = 5.0 * period

    first_panel_relative_length = dt * velocity / chord_length

    wing_builder = WingBuilder(
        section_points = [
            SpatialVector(z=-span/2.0),
            SpatialVector(z=span/2.0)
        ],
        chord_vectors = [
            SpatialVector(x=chord_length),
            SpatialVector(x=chord_length)
        ],
        section_model = SectionModel(model=Foil()),
    )

    line_force_model = LineForceModelBuilder()
    line_force_model.add_wing_builder(wing_builder)

    simulation_builder_quasi_steady = SimulationBuilder(
        line_force_model = line_force_model,
        simulation_settings = QuasiSteadySettings()
    )

    simulation_builder_dynamic = SimulationBuilder(
        line_force_model = line_force_model,
        simulation_settings = DynamicSettings(
            wake = DynamicWakeBuilder(
                first_panel_relative_length = first_panel_relative_length
            )
        )
    )



    sim_builder_list = [
        simulation_builder_quasi_steady,
        simulation_builder_dynamic
    ]

    label_list = ["Quasi-steady", "Dynamic"]
    color_list = [default_colors[0], default_colors[2]]

    w_plot = 14
    fig = plt.figure(figsize=(w_plot, w_plot / 3.0))

    max_cl = []

    for builder, label, color in zip(
        sim_builder_list,
        label_list,
        color_list
    ):
        print("Running ", label, "simulations:")

        simulation = Simulation(builder.to_json_string())

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
            freestream_velocity.append([velocity, 0.0, 0.0])

        start_time = time_func.time()
        while t < final_time:
            print("Running sim at time = ", t)
            simulation.set_translation_with_velocity_using_finite_difference(
                [0.0, position_func(t), 0.0],
                dt
            )

            result = simulation.do_step(
                time = t,
                time_step = dt,
                freestream_velocity = freestream_velocity,
            )

            forces = result.integrated_forces_sum()

            time.append(t)
            lift.append(forces[1] / force_factor)
            drag.append(forces[0] / force_factor)


            print("Number of iterations: ", result.iterations)

            t += dt

        end_time = time_func.time()

        elapsed_time = end_time - start_time
        print("Elapsed time: ", elapsed_time)
        print("Time speed up: ", final_time / elapsed_time)

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
        label="Simplified quasi-steady * Real(Theodorsen)", linestyle="--", color=color_list[1]
    )

    plt.plot(
        time,
        cl_quasi_steady_theory,
        label="Simplified quasi-steady", linestyle="--", color=color_list[0]
    )

    print("Theodorsen ratio", theodorsen_reduction)
    #print("CL ratio", max_cl[0] / max_cl[1])

    plt.xlim(1.0 * period, 5 * period)
    plt.ylim(-1.1 * max_cl_theory, 1.1 * max_cl_theory)
    plt.xlabel("Time")
    plt.ylabel("Lift coefficient")
    plt.legend()

    plt.tight_layout()
    plt.savefig("heaving_wing.png", dpi=300, bbox_inches="tight")

    plt.show()
