"""
Compares the default rotor sail model RotorSailTillig2020 in stormbird_setup against the empirical
model it is fitted to, from:

Tillig, F., Ringsberg, J. W. (2020). Design, operation and analysis of wind-assisted cargo ships.
Ocean Engineering, 211, 107603. https://doi.org/10.1016/j.oceaneng.2020.107603

The empirical model gives the lift, drag and power coefficients of a complete rotor, with an aspect
ratio of 6 and an end disc with twice the diameter of the rotor, as polynomials in the spin ratio
(equations 7, 8 and 9 in the paper). The default sail model is simulated as a single rotor with the
same aspect ratio, standing on a symmetry plane, which is the setup used when the lift was fitted.
The RotorSailDeybach2024 model is included for comparison.

The lift parameters are fitted to the lift polynomial. The drag is fitted as a function of the lift
coefficient, which is what matters for the performance of the sail, and is therefore plotted against
the lift coefficient. The end disc is modelled with a virtual extension of the span above the rotor,
which reduces the induced drag. The length of the extension and the section drag parameters are
fitted together. The power coefficient in Stormbird is the same polynomial as in the paper.
"""

import numpy as np
import matplotlib.pyplot as plt

from stormbird_setup.default_sails import DefaultSail
from stormbird_setup.lifting_line import CompleteSailModelBuilder, SymmetryCondition
from stormbird_setup.spatial_vector import SpatialVector
from stormbird_setup.utils import revolutions_per_second_from_spin_ratio

from pystormbird.lifting_line import CompleteSailModel
from pystormbird.wind import WindCondition

DIAMETER = 5.0
ASPECT_RATIO = 6.0
HEIGHT = ASPECT_RATIO * DIAMETER
WIND_VELOCITY = 8.0
DENSITY = 1.225

# Coefficients of the polynomials in the paper, in decreasing order, as used by np.polyval
TILLIG_LIFT = [-0.0046, 0.1145, -0.9817, 3.1309, -0.1039, 0.0]
TILLIG_DRAG = [-0.0017, 0.0464, -0.4424, 1.7243, -1.641, 0.6375]
TILLIG_POWER = [0.0001, -0.0004, 0.0143, -0.0168, 0.0234, 0.0]


def simulate_coefficients(default_sail: DefaultSail, spin_ratios: np.ndarray) -> dict:
    '''
    Lift, drag and power coefficients of a single rotor, as a function of the spin ratio, based on
    the projected area of the rotor
    '''
    model_builder = CompleteSailModelBuilder.new_from_default_sail(
        sail = default_sail,
        positions = [SpatialVector(x=0.0, y=0.0, z=0.0)],
        chord_length = DIAMETER,
        height = HEIGHT,
        symmetry_condition = SymmetryCondition.Z,
    )

    model = CompleteSailModel(model_builder.to_json_string())

    wind_condition = WindCondition.new_constant(
        direction_coming_from = 0.0,
        velocity = WIND_VELOCITY
    )

    area = DIAMETER * HEIGHT
    force_factor = 0.5 * DENSITY * area * WIND_VELOCITY**2
    power_factor = force_factor * WIND_VELOCITY

    cl = np.zeros_like(spin_ratios)
    cd = np.zeros_like(spin_ratios)
    cp = np.zeros_like(spin_ratios)

    for index, spin_ratio in enumerate(spin_ratios):
        model.set_internal_state([
            revolutions_per_second_from_spin_ratio(
                spin_ratio = spin_ratio,
                diameter = DIAMETER,
                velocity = WIND_VELOCITY
            )
        ])

        result = model.do_step(
            time = 0,
            time_step = 1,
            wind_condition = wind_condition,
            ship_velocity = 0.0
        )

        forces = result.integrated_forces_sum()

        cd[index] = forces[0] / force_factor
        cl[index] = abs(forces[1]) / force_factor
        cp[index] = result.input_power_sum() / power_factor

    return {"cl": cl, "cd": cd, "cp": cp}


if __name__ == "__main__":
    spin_ratios = np.linspace(0.0, 5.0, 51)

    models = {
        "RotorSailTillig2020": simulate_coefficients(DefaultSail.RotorSailTillig2020, spin_ratios),
        "RotorSailDeybach2024": simulate_coefficients(DefaultSail.RotorSailDeybach2024, spin_ratios),
    }

    empirical = {
        "cl": np.polyval(TILLIG_LIFT, spin_ratios),
        "cd": np.polyval(TILLIG_DRAG, spin_ratios),
        "cp": np.polyval(TILLIG_POWER, spin_ratios),
    }

    fig, axes = plt.subplots(1, 3, figsize = (15, 5))

    # (x, y) for each plot: lift against spin ratio, drag against lift, and power against spin ratio
    plots = [
        ("sr", "cl"),
        ("cl", "cd"),
        ("sr", "cp"),
    ]

    labels = {
        "sr": "Spin ratio",
        "cl": "Lift coefficient",
        "cd": "Drag coefficient",
        "cp": "Power coefficient",
    }

    empirical["sr"] = spin_ratios

    for coefficients in models.values():
        coefficients["sr"] = spin_ratios

    for ax, (x_key, y_key) in zip(axes, plots):
        ax.plot(empirical[x_key], empirical[y_key], "k", linewidth = 2, label = "Tillig and Ringsberg (2020), empirical model")

        for name, coefficients in models.items():
            linestyle = "-" if name == "RotorSailTillig2020" else "--"
            ax.plot(coefficients[x_key], coefficients[y_key], linestyle, label = f"Stormbird, {name}")

        ax.set_xlabel(labels[x_key])
        ax.set_ylabel(labels[y_key])
        ax.set_xlim(left = 0)
        ax.set_ylim(bottom = 0)
        ax.grid(True)

    handles, legend_labels = axes[0].get_legend_handles_labels()
    fig.legend(handles, legend_labels, loc = "lower center", ncol = 3)

    plt.tight_layout(rect = (0, 0.08, 1, 1))

    plt.savefig("compare_tillig_empirical_model.pdf")
    plt.savefig("compare_tillig_empirical_model.png", dpi = 200)

    plt.show()
