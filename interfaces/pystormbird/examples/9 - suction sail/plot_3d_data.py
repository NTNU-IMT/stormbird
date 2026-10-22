'''
Step 2 in the suction sail example: simulates the three-dimensional model from the paper, and
compares the results to the three-dimensional data, using the section model fitted to the
two-dimensional data in the first step.

TODO: When the retuning in retune_section_model.py is updated to the slope reduction in the foil
model, and `USE_RETUNED_SECTION_MODEL` is set in suction_sail_model.py, the retuned model is
compared as well.

The suction coefficient is set directly as the internal state of the wing, which gives the same
suction coefficient along the whole span.

The model in the paper had an aspect ratio of 4, and was standing directly on the wall of the wind
tunnel. This is modeled with a symmetry plane at the bottom of the wing, which gives an effective
aspect ratio of 8 (see `create_simulation` in suction_sail_model.py).

The largest suction coefficient in the three-dimensional data, 0.05, is slightly larger than the
largest suction coefficient in the two-dimensional data, 0.047. The model fitted to the
two-dimensional data uses the values for the largest suction coefficient in the data for larger
values.
'''

import numpy as np
import matplotlib.pyplot as plt

from suction_sail_data import CQ_VALUES_3D, lift_data, drag_data
from suction_sail_model import (
    create_simulation,
    suction_coefficient_direct,
    angle_sweep,
    load_raw_varying_foil,
    load_retuned_varying_foil,
    USE_RETUNED_SECTION_MODEL,
)

if __name__ == "__main__":
    models = {
        "fitted to 2D data": (load_raw_varying_foil(), "--"),
    }

    if USE_RETUNED_SECTION_MODEL:
        models["retuned to 3D data"] = (load_retuned_varying_foil(), "-")

    angles_of_attack_deg = np.arange(0.0, 45.5, 0.5)

    w_plot = 14
    h_plot = w_plot / 2.35

    fig = plt.figure(figsize = (w_plot, h_plot))
    ax_cl = fig.add_subplot(121)
    ax_cd = fig.add_subplot(122)

    for cq in CQ_VALUES_3D:
        aoa_data, cl_data = lift_data(cq, "3d")
        cl_drag_data, cd_data = drag_data(cq, "3d")

        color = ax_cl.plot(aoa_data, cl_data, "o", markersize=3, label=f"Data, Cq = {cq}")[0].get_color()
        ax_cd.plot(cl_drag_data, cd_data, "o", markersize=3, color=color)

        for label, (varying_foil, linestyle) in models.items():
            simulation = create_simulation(suction_coefficient_direct(), varying_foil)

            results = angle_sweep(simulation, angles_of_attack_deg, cq)

            cl_model = [result.cl for result in results]
            cd_model = [result.cd for result in results]

            ax_cl.plot(angles_of_attack_deg, cl_model, linestyle, color=color)
            ax_cd.plot(cl_model, cd_model, linestyle, color=color)

    # Legend entries for the line styles
    for label, (_, linestyle) in models.items():
        ax_cl.plot([], [], linestyle, color="gray", label=f"Lifting line, {label}")

    ax_cl.legend()

    ax_cl.set_xlabel("Angle of attack [deg]")
    ax_cl.set_ylabel("Lift coefficient (CL)")

    ax_cd.set_xlabel("Lift coefficient (CL)")
    ax_cd.set_ylabel("Drag coefficient (CD)")

    plt.tight_layout()
    plt.show()
