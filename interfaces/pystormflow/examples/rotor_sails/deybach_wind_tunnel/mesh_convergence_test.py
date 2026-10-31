'''
Mesh convergence test for the Stormflow simulations in run_single_rotor.py. A single spin ratio is
run with increasing resolution at the rotor, starting with the lowest. By default, only the
recommended variant of the actuator line model is tested. Use --variant to choose one or more other
variants, or all of them:

    python mesh_convergence_test.py --variant lifting_line
    python mesh_convergence_test.py --variant all --cells-per-chord-length 4 8 16

The force history of each case is stored as a time series in time_series_mesh_convergence_stormflow,
and the results are plotted with plotting.py:

    python plotting.py mesh-convergence --spin-ratio 3.0

Note that the uniform grid in Stormflow means that the number of cells increases with the cube of
the resolution, so the highest resolutions require a lot of memory.
'''

import argparse

from stormbird_setup.stormflow import ComputePlatform

import rotor_setup
from plotting import (
    Solver,
    mean_coefficients,
    mesh_convergence_time_series_path,
    write_time_series,
)
from run_single_rotor import simulate_force_history

if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--spin-ratio", type=float, default=rotor_setup.MESH_TEST_SPIN_RATIO,
        help="Spin ratio to test"
    )
    parser.add_argument(
        "--cells-per-chord-length", type=int, nargs="+",
        default=rotor_setup.MESH_TEST_CELLS_PER_CHORD_LENGTH, help="Resolutions to test"
    )
    rotor_setup.add_variant_argument(parser, "test")
    parser.add_argument("--cpu", action="store_true", help="Run on the CPU instead of the GPU")

    args = parser.parse_args()

    compute_platform = ComputePlatform.CPU if args.cpu else ComputePlatform.GPU

    variants = rotor_setup.variants_from_arguments(args.variant)

    for cells_per_chord_length in sorted(args.cells_per_chord_length):
        for variant in variants:
            print(f"{variant.label}, cells per chord length: {cells_per_chord_length}")

            force_history = simulate_force_history(
                args.spin_ratio, variant, compute_platform, cells_per_chord_length
            )

            write_time_series(
                force_history,
                mesh_convergence_time_series_path(
                    variant, args.spin_ratio, cells_per_chord_length, Solver.Stormflow
                )
            )

            coefficients = mean_coefficients(force_history)

            print(f"cl = {coefficients['cl']:.3f}, cd = {coefficients['cd']:.3f}", flush=True)
