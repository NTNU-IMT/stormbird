'''
Compares the power curve of the default suction sail model in stormbird_setup,
DefaultSail.SuctionSailCharrier1985, to the experiments.

The model is used directly, with the input power to the fan as the internal state, as in a complete
sail model. As in the tuning, it is assumed that the sail is operated at the maximum lift for each
power, and the geometry is the wind tunnel model from the paper. Both the version for the iterative
solver, which the model was tuned with, and the version for the linearized solver are compared.
'''

import numpy as np
import matplotlib.pyplot as plt

from stormbird_setup.default_sails import DefaultSail, SolverType

from suction_sail_data import power_curve_data
from suction_sail_model import SuctionSailModel

VELOCITY = 10.0

# Coarse sweep for the maximum lift, which occurs between 15 and 40 degrees for all suction
# coefficients in the model. The angle is refined around the best value.
ANGLES_OF_ATTACK_DEG = np.arange(10.0, 49.0, 2.0)

if __name__ == "__main__":
    cp_data, cl_data = power_curve_data()

    fig = plt.figure(figsize = (8, 5.5))
    ax = fig.add_subplot(111)

    ax.plot(cp_data, cl_data, "o--", color = "black", label = "Experiments")

    cp_model = np.geomspace(0.5 * cp_data[0], cp_data[-1], 14)

    for solver_type in [SolverType.Iterative, SolverType.Linearized]:
        model = SuctionSailModel(
            sail_definition = DefaultSail.SuctionSailCharrier1985.definition(solver_type)
        )

        power_factor = 0.5 * model.density * model.area * VELOCITY**3

        cl_model = np.zeros(len(cp_model))

        print(f"{solver_type.value} solver:")

        for i, cp in enumerate(cp_model):
            result = model.max_lift_case(
                velocity = VELOCITY,
                angles_of_attack_deg = ANGLES_OF_ATTACK_DEG,
                internal_state = cp * power_factor
            )

            cl_model[i] = result.cl

            print(
                f"  Cp = {cp:.4f}: CL max = {result.cl:.3f}, "
                f"wing Cq = {result.wing_suction_coefficient(VELOCITY):.4f}"
            )

        ax.plot(cp_model, cl_model, "-", label = f"Default sail model, {solver_type.value} solver")

    ax.set_xlabel("Power coefficient (CP)")
    ax.set_ylabel("Maximum lift coefficient (CL)")

    ax.legend()

    plt.tight_layout()
    plt.savefig("default_sail_power_curve.png", dpi = 150)
    plt.show()
