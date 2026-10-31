'''
Mesh convergence test for the OpenFOAM simulations in run_single_rotor.py. This is the OpenFOAM
version of mesh_convergence_test.py in interfaces/pystormflow/examples/rotor_sails/deybach_wind_tunnel,
and has the same settings and options. A single spin ratio is run with increasing resolution at the rotors, starting
with the lowest. The background mesh is refined together with the resolution at the rotors, so that
the ratio between them is the same for all resolutions. By default, only the recommended variant of
the actuator line model is tested. Use --variant to choose one or more other variants, or all of
them:

    python mesh_convergence_test.py --variant lifting_line
    python mesh_convergence_test.py --variant all --cells-per-chord-length 4 8

The force history of each case is stored as a time series in time_series_mesh_convergence_openfoam,
and the results are plotted with plot_results.py:

    python plot_results.py mesh-convergence --spin-ratio 3.0
'''

import argparse
import sys
from pathlib import Path

sys.path.append(str(
    Path(__file__).resolve().parents[5] / "pystormflow" / "examples" / "rotor_sails" /
    "deybach_wind_tunnel"
))

import rotor_setup
from plotting import (
    Solver,
    mean_coefficients,
    mesh_convergence_time_series_path,
    write_time_series,
)

from run_single_rotor import case_folder, simulate_force_history

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
    parser.add_argument(
        "--nr-processes", type=int, default=None,
        help="Number of processes for the solver. The default is the number of physical cores."
    )

    args = parser.parse_args()

    variants = rotor_setup.variants_from_arguments(args.variant)

    for cells_per_chord_length in sorted(args.cells_per_chord_length):
        for variant in variants:
            print(f"{variant.label}, cells per chord length: {cells_per_chord_length}")

            file_path = mesh_convergence_time_series_path(
                variant, args.spin_ratio, cells_per_chord_length, Solver.OpenFOAM
            )

            force_history = simulate_force_history(
                case_folder(file_path),
                args.spin_ratio,
                variant,
                args.nr_processes,
                cells_per_chord_length
            )

            write_time_series(force_history, file_path)

            coefficients = mean_coefficients(force_history)

            print(f"cl = {coefficients['cl']:.3f}, cd = {coefficients['cd']:.3f}", flush=True)
