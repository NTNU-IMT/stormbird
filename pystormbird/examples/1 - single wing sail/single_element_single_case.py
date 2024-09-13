import numpy as np
import matplotlib.pyplot as plt

import argparse

from simulation import SimulationCase, SimulationMode
from single_element_foil_section import get_foil_model

from enum import Enum

class TestCases(Enum):
    RAW_SIMULATION = 0
    PRESCRIBED_CIRCULATION = 1
    INITIALIZED_SIMULATION = 2

    def to_string(self):
        return self.name.replace("_", " ").lower()

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Run a single case")
    parser.add_argument("--angle-of-attack", type=float, default = 0.0, help="Angle of attack in degrees")

    args = parser.parse_args()

    simulation_mode = SimulationMode.STATIC

    foil_model = get_foil_model()

    section_model_dict = {
        "Foil": foil_model.__dict__
    }

    cases = [TestCases.RAW_SIMULATION, TestCases.PRESCRIBED_CIRCULATION, TestCases.INITIALIZED_SIMULATION]

    w_plot = 16
    fig = plt.figure(figsize=(w_plot, w_plot/3.0))
    ax1 = fig.add_subplot(131)
    ax2 = fig.add_subplot(132)
    ax3 = fig.add_subplot(133)

    for case in cases:
        print()
        print(case.to_string())

        match case:
            case TestCases.RAW_SIMULATION:
                prescribed_circulation    = False
                prescribed_initialization = False
            case TestCases.PRESCRIBED_CIRCULATION:
                prescribed_circulation    = True
                prescribed_initialization = False
            case TestCases.INITIALIZED_SIMULATION:
                prescribed_circulation    = False
                prescribed_initialization = True
            case _:
                raise ValueError("Invalid case")


        sim_case = SimulationCase(
            angle_of_attack = args.angle_of_attack,
            section_model_dict = section_model_dict,
            simulation_mode = SimulationMode.STATIC,
            prescribed_circulation = prescribed_circulation,
            prescribed_initialization = prescribed_initialization,
            z_symmetry=True
        )

        result_history = sim_case.run()

        cd = np.zeros(len(result_history))
        cl = np.zeros(len(result_history))

        for i in range(len(result_history)):
            force = result_history[i].integrated_forces[0].total

            cd[i] = force.x / sim_case.force_factor
            cl[i] = force.y / sim_case.force_factor

        print('Last lift coefficient:', cl[-1])
        print('Last drag coefficient:', cd[-1])

        circulation_strength = np.array(result_history[-1].force_input.circulation_strength)
        angles_of_attack     = np.array(result_history[-1].force_input.angles_of_attack)

        plt.sca(ax1)
        if len(cl) > 1:
            plt.plot(cl)
        else:
            plt.plot([0, 1], [cl[0], cl[0]])

        plt.sca(ax2)
        plt.plot(-circulation_strength, label=case.to_string())

        plt.sca(ax3)
        plt.plot(np.degrees(angles_of_attack))

    ax2.legend()

    plt.show()
