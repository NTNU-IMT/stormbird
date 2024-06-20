import numpy as np

import matplotlib.pyplot as plt
import json

from pystormbird.line_force_model import LineForceModel

if __name__ == '__main__':
    line_force_builder = {
        "wing_builders": [
            {
                "section_points": [
                    {"x": 0.0, "y": 0.0, "z": -5.0},
                    {"x": 0.0, "y": 0.0, "z": 5.0}
                ],
                "chord_vectors": [
                    {"x": 1.0, "y": 0.0, "z": 0.0},
                    {"x": 1.0, "y": 0.0, "z": 10.0}
                ],
                "section_model": {
                    "Foil": {}
                }
            }
        ],
        "nr_sections": 256,
    }

    noise_amplitude = 0.1

    line_force_model = LineForceModel(json.dumps(line_force_builder))

    relative_span_distance = np.array(line_force_model.relative_span_distance())

    circulation_org = np.sqrt(1.0 - (relative_span_distance/0.5)**2)

    circulation_noisy = circulation_org + noise_amplitude * np.random.randn(len(relative_span_distance))

    plt.plot(relative_span_distance, circulation_org, label="Original")
    plt.plot(relative_span_distance, circulation_noisy, label="With noise", alpha=0.5)

    viscosity = [0.1]

    for visc in viscosity:
        circulation_smoothed = line_force_model.circulation_strength_with_viscosity(
            noisy_strength = circulation_org,
            viscosity = visc,
            iterations = 2
        )

        plt.plot(relative_span_distance, circulation_smoothed, label=f"Smoothed with viscosity {visc}")

    plt.xlabel("Relative span distance")
    plt.ylabel("Circulation")

    plt.legend()

    plt.savefig('../figures/circulation_viscosity_example.png', dpi=300)

    plt.show()