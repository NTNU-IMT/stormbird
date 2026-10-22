

import numpy as np
import matplotlib.pyplot as plt

from suction_sail_data import CQ_VALUES_3D, lift_data, drag_data
from suction_sail_model import SuctionSailModel

if __name__ == "__main__":
    angles_of_attack_deg = np.arange(-15.0, 45.5, 0.5)

    w_plot = 14
    h_plot = w_plot / 2.35

    fig = plt.figure(figsize = (w_plot, h_plot))
    ax_cl = fig.add_subplot(121)
    ax_cd = fig.add_subplot(122)

    model = SuctionSailModel()

    velocity = 10.0

    for cq in CQ_VALUES_3D:
        aoa_data, cl_data = lift_data(cq, "3d")
        cl_drag_data, cd_data = drag_data(cq, "3d")

        model_results = model.angle_sweep(
            velocity = velocity, 
            angles_of_attack_deg = angles_of_attack_deg, 
            internal_state = cq
        )

        cl_model = [result.cl for result in model_results]
        cd_model = [result.cd for result in model_results]

        color = ax_cl.plot(aoa_data, cl_data, "--", label=f"Cq = {cq:.3f}, Exp")[0].get_color()
        ax_cd.plot(cl_drag_data, cd_data, "--", color=color)

        ax_cl.plot(angles_of_attack_deg, cl_model, color=color, label=f"Cq = {cq:.3f}, LL")
        ax_cd.plot(cl_model, cd_model, color=color)

    ax_cl.legend()

    ax_cl.set_xlabel("Angle of attack [deg]")
    ax_cl.set_ylabel("Lift coefficient (CL)")

    ax_cd.set_xlabel("Lift coefficient (CL)")
    ax_cd.set_ylabel("Drag coefficient (CD)")

    plt.tight_layout()
    plt.show()