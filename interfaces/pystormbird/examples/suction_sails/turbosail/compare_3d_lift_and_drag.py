'''
Compares the lift and drag of the default suction sail model in stormbird_setup to the 3D data, with
the suction coefficient as the internal state. Both the version for the iterative solver, which the
model was tuned with, and the version for the linearized solver are compared.
'''

import numpy as np
import matplotlib.pyplot as plt

from stormbird_setup.default_sails import DefaultSail, SolverType

from suction_sail_data import CQ_VALUES_3D, lift_data, drag_data
from suction_sail_model import SuctionSailModel, SUCTION_COEFFICIENT_DIRECT

LINE_STYLES = {
    SolverType.Iterative: "-",
    SolverType.Linearized: ":",
}

if __name__ == "__main__":
    angles_of_attack_deg = np.arange(-15.0, 45.5, 0.5)

    w_plot = 14
    h_plot = w_plot / 2.35

    fig = plt.figure(figsize = (w_plot, h_plot))
    ax_cl = fig.add_subplot(121)
    ax_cd = fig.add_subplot(122)

    models = {
        solver_type: SuctionSailModel(
            sail_definition = DefaultSail.SuctionSailCharrier1985.definition(solver_type),
            internal_state_conversion = SUCTION_COEFFICIENT_DIRECT
        ) for solver_type in LINE_STYLES
    }

    velocity = 10.0

    for cq in CQ_VALUES_3D:
        aoa_data, cl_data = lift_data(cq, "3d")
        cl_drag_data, cd_data = drag_data(cq, "3d")

        color = ax_cl.plot(aoa_data, cl_data, "--", label=f"Cq = {cq:.3f}")[0].get_color()
        ax_cd.plot(cl_drag_data, cd_data, "--", color=color)

        for solver_type, model in models.items():
            model_results = model.angle_sweep(
                velocity = velocity,
                angles_of_attack_deg = angles_of_attack_deg,
                internal_state = cq
            )

            cl_model = [result.cl for result in model_results]
            cd_model = [result.cd for result in model_results]

            ax_cl.plot(angles_of_attack_deg, cl_model, LINE_STYLES[solver_type], color=color)
            ax_cd.plot(cl_model, cd_model, LINE_STYLES[solver_type], color=color)

    # Legend entries for the line styles
    ax_cl.plot([], [], "--", color="grey", label="Experiments")

    for solver_type, line_style in LINE_STYLES.items():
        ax_cl.plot([], [], line_style, color="grey", label=f"LL, {solver_type.value} solver")

    ax_cl.legend()

    ax_cl.set_xlabel("Angle of attack [deg]")
    ax_cl.set_ylabel("Lift coefficient (CL)")

    ax_cd.set_xlabel("Lift coefficient (CL)")
    ax_cd.set_ylabel("Drag coefficient (CD)")

    plt.tight_layout()
    plt.show()
