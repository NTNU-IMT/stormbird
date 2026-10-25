'''
Mesh convergence test for the Stormflow simulations in run_simulations.py. A single spin ratio is run
for all variants of the actuator line model, with increasing resolution at the rotor, starting with
the lowest. The results are plotted every time a case is finished. The OpenFOAM version of this
example has the same test.

Note that the uniform grid in Stormflow means that the number of cells increases with the cube of
the resolution, so the highest resolutions require a lot of memory.
'''

import argparse
from pathlib import Path

from stormbird_setup.stormflow import ComputePlatform

from run_simulations import simulate_force_history, rotor_setup, Variant

RESULTS_PATH = Path(__file__).parent / "mesh_convergence_stormflow.json"


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--spin-ratio", type=float, default=rotor_setup.MESH_TEST_SPIN_RATIO, help="Spin ratio to test"
    )
    parser.add_argument(
        "--cells-per-chord-length", type=int, nargs="+",
        default=rotor_setup.MESH_TEST_CELLS_PER_CHORD_LENGTH, help="Resolutions to test"
    )
    parser.add_argument(
        "--variants", type=str, nargs="+", default=[variant.value for variant in Variant],
        choices=[variant.value for variant in Variant], help="Variants of the actuator line model to test"
    )
    parser.add_argument("--cpu", action="store_true", help="Run on the CPU instead of the GPU")
    parser.add_argument("--plot-only", action="store_true", help="Only plot the stored results")

    args = parser.parse_args()

    compute_platform = ComputePlatform.CPU if args.cpu else ComputePlatform.GPU

    plot_path = Path(__file__).parent / "mesh_convergence_stormflow.png"

    # Results from earlier runs are kept, so that individual cases can be rerun
    results = rotor_setup.read_results(RESULTS_PATH)

    if not args.plot_only:
        for cells_per_chord_length in sorted(args.cells_per_chord_length):
            for variant in map(Variant, args.variants):
                print(f"{variant.label}, cells per chord length: {cells_per_chord_length}")

                force_history = simulate_force_history(
                    args.spin_ratio, variant, compute_platform, cells_per_chord_length
                )

                coefficients = rotor_setup.force_coefficients(
                    force_history["time"].tolist(),
                    force_history["force.x"].tolist(),
                    force_history["force.y"].tolist()
                )

                print(f"cl = {coefficients['cl']:.3f}, cd = {coefficients['cd']:.3f}", flush=True)

                results.setdefault(variant.value, {})[str(cells_per_chord_length)] = coefficients

                rotor_setup.write_results(results, RESULTS_PATH)
                rotor_setup.plot_mesh_convergence(results, args.spin_ratio, "Stormflow", plot_path)

    rotor_setup.plot_mesh_convergence(results, args.spin_ratio, "Stormflow", plot_path)
