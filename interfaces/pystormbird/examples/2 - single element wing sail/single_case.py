import argparse

import numpy as np
import matplotlib.pyplot as plt

from stormbird_setup.simplified_setup.single_wing_simulation import SingleWingSimulation, SolverType
from stormbird_setup.direct_setup.section_models import SectionModel, Foil

from pystormbird.lifting_line import Simulation

def simulate_single_case(
    *,
    angle_of_attack_deg: float,
    dynamic: bool = False,
    solver_type: SolverType = SolverType.Linearized,
    smoothing_length: float = 0.0,
) -> dict:
    
    section_model = SectionModel(
        model = Foil(
            cl_initial_slope = 5.794665, # From example 1 - section models which indicate slightly below 2*pi for the slope
            cd_min = 0.0186,
            cd_second_order_factor=1.052354,
            mean_positive_stall_angle=np.radians(12.7), # Stall tuned based on 3D data, not sectional model. Generally necessary
            stall_range=np.radians(7.0),
        )
    )

    chord_length = 1.0
    height = 4.5
    velocity = 8.0
    density = 1.225

    force_factor = 0.5 * chord_length * height * density * velocity**2

    dynamic_end_time = 40 * chord_length / velocity
    dynamic_time_step = 0.25 * chord_length / velocity

    sim_settings = SingleWingSimulation(
        chord_length=1.0,
        height=4.5,
        section_model=section_model,
        dynamic=dynamic,
        solver_type=solver_type,
        z_symmetry=True,
        smoothing_length=smoothing_length,
    )

    simulation = Simulation(
        sim_settings.get_simulation_builder().to_json_string()
    )

    freestream_velocity_points = simulation.get_freestream_velocity_points()

    nr_points = len(freestream_velocity_points)

    freestream_velocity_list = []
    for _ in freestream_velocity_points:
        freestream_velocity_list.append(
            [velocity, 0.0, 0.0]
        )

    current_time = 0.0

    simulation.set_local_wing_angles([-np.radians(angle_of_attack_deg)])

    result_history = []
    if dynamic:
        while current_time < dynamic_end_time:
            result = simulation.do_step(
                time = current_time, 
                time_step = dynamic_time_step, 
                freestream_velocity = freestream_velocity_list
            )

            current_time += dynamic_time_step

            result_history.append(result)
    else:
        result = simulation.do_step(
            time = 0.0, 
            time_step = 1.0, 
            freestream_velocity = freestream_velocity_list
        )

        result_history.append(result)

    force = result_history[-1].integrated_forces[0].total

    cd = force[0] / force_factor
    cl = force[1] / force_factor

    out = {
        "cl": cl,
        "cd": cd,
        "circulation_strength": np.array(result_history[-1].force_input.circulation_strength),
        "angles_of_attack": np.array(result_history[-1].force_input.angles_of_attack),
    }

    return out

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Run a single case")
    parser.add_argument("--angle-of-attack", type=float, default = 5.0, help="Angle of attack in degrees")

    args = parser.parse_args()

    w_plot = 16
    fig = plt.figure(figsize=(w_plot, w_plot/2.35))
    ax_circulation = fig.add_subplot(121)
    ax_angle = fig.add_subplot(122)

    dynamic = [False, False, False, True]
    solver_types = [SolverType.Linearized, SolverType.SimpleIterative, SolverType.SimpleIterative, SolverType.SimpleIterative]
    smoothing_length = [0.0, 0.0, 0.1, 0.1]

    for dyn, solver, smoothing in zip(dynamic, solver_types, smoothing_length):
        label = "Dynamic" if dyn else "Quasi-steady"
        label += " - " + solver.name

        if smoothing > 0.0:
            label += f", smoothing {smoothing:.1f}"

        print("Running simulation case:", label)

        res = simulate_single_case(
            angle_of_attack_deg = args.angle_of_attack,
            solver_type = solver,
            dynamic = dyn,
            smoothing_length = smoothing
        )

        print('Lift coefficient:', res['cl'])
        print('Drag coefficient:', res['cd'])

        if solver == SolverType.SimpleIterative:
            linestyle='--'
        else:
            linestyle='-'


        ax_circulation.plot(-res['circulation_strength'], label=label, linestyle=linestyle)

        ax_angle.plot(np.degrees(res['angles_of_attack']), label=label, linestyle=linestyle)

    ax_angle.plot([0, len(res['angles_of_attack'])], [args.angle_of_attack, args.angle_of_attack], 'k--', label='Geometric angle of attack')


    ax_circulation.set_xlabel('Line model segment')
    ax_circulation.set_ylabel('Circulation strength')

    ax_angle.set_xlabel('Line model segment')
    ax_angle.set_ylabel('Effective angle of attack [deg]')

    ax_angle.legend()

    plt.show()
