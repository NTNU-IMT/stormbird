import argparse

import numpy as np
import matplotlib.pyplot as plt

from stormbird_setup.simplified_setup.single_wing_simulation import SingleWingSimulation, SolverType
from stormbird_setup.direct_setup.section_models import SectionModel, RotatingCylinder
from stormbird_setup.direct_setup.lifting_line.velocity_corrections import VelocityCorrections, VelocityCorrectionType

from pystormbird.lifting_line import Simulation

def get_section_model():
    '''
    Returns the section model
    '''

    return SectionModel(
        model = RotatingCylinder()
    )

def revolutions_per_second_from_spin_ratio(
    *,
    spin_ratio: float,
    diameter: float,
    velocity: float
):
    circumference = np.pi * diameter
    tangential_velocity = velocity * spin_ratio
            
    revolutions_per_second = -tangential_velocity / circumference 

    return revolutions_per_second


def simulate_single_case(
    *,
    spin_ratio: float,
    solver_type: SolverType = SolverType.SimpleIterative,
    max_induced_velocity_ratio: float = 1.0,
) -> dict:

    diameter = 5.0
    height = 35.0
    velocity = 8.0
    density = 1.225

    force_factor = 0.5 * diameter * height * density * velocity**2

    sim_settings = SingleWingSimulation(
        chord_length=diameter,
        height=height,
        section_model=get_section_model(),
        solver_type=solver_type,
        z_symmetry=True,
    )

    simulation_builder = sim_settings.get_simulation_builder()

    if max_induced_velocity_ratio > 0.0:
        simulation_builder.simulation_settings.solver.velocity_corrections = VelocityCorrections(
            type = VelocityCorrectionType.MaxInducedVelocityMagnitudeRatio,
            value = max_induced_velocity_ratio
        )

    simulation = Simulation(
        simulation_builder.to_json_string()
    )

    freestream_velocity_points = simulation.get_freestream_velocity_points()

    freestream_velocity_list = []
    for _ in freestream_velocity_points:
        freestream_velocity_list.append(
            [velocity, 0.0, 0.0]
        )


    section_model_internal_state = revolutions_per_second_from_spin_ratio(
        spin_ratio=spin_ratio,
        diameter=diameter,
        velocity=velocity
    )

    simulation.set_section_models_internal_state([section_model_internal_state])

    result = simulation.do_step(
        time = 0.0, 
        time_step = 1.0, 
        freestream_velocity = freestream_velocity_list
    )

    force = result.integrated_forces[0].total

    cd = force[0] / force_factor
    cl = force[1] / force_factor

    out = {
        "cl": cl,
        "cd": cd,
        "circulation_strength": np.array(result.force_input.circulation_strength),
        "angles_of_attack": np.array(result.force_input.angles_of_attack),
    }

    return out

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Run a single case")
    parser.add_argument("--spin-ratio", type=float, default = 4.0, help="Spin ratio")

    args = parser.parse_args()

    w_plot = 16
    fig = plt.figure(figsize=(w_plot, w_plot/2.35))
    ax_circulation = fig.add_subplot(121)
    ax_angle = fig.add_subplot(122)

    max_induced_velocity_ratios = [0.0, 1.0, 1.0]
    solver_types = [SolverType.SimpleIterative, SolverType.SimpleIterative, SolverType.Linearized]


    for max_induced_velocity_ratio, solver in zip(max_induced_velocity_ratios, solver_types):
        label = solver.name

        if max_induced_velocity_ratio == 0.0:
            label += ", no induced velocity limit"
        else:
            label += f", max induced velocity ratio {max_induced_velocity_ratio:.1f}"

        print("Running simulation case:", label)

        res = simulate_single_case(
            spin_ratio=args.spin_ratio,
            solver_type = solver,
            max_induced_velocity_ratio = max_induced_velocity_ratio
        )

        print('Lift coefficient:', res['cl'])
        print('Drag coefficient:', res['cd'])


        ax_circulation.plot(-res['circulation_strength'], label=label)

        ax_angle.plot(np.degrees(res['angles_of_attack']), label=label)

    ax_circulation.set_xlabel('Line model segment')
    ax_circulation.set_ylabel('Circulation strength')

    ax_angle.set_xlabel('Line model segment')
    ax_angle.set_ylabel('Effective angle of attack [deg]')

    ax_angle.legend()

    plt.show()
