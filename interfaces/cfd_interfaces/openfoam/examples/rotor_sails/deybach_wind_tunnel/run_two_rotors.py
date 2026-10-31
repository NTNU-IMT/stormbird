'''
Runs actuator line simulations of two rotor sails with OpenFOAM, where the interfering rotor is
placed at different angular positions around the rotor at the origin. This is the OpenFOAM version
of run_two_rotors.py in interfaces/pystormflow/examples/rotor_sails/deybach_wind_tunnel, and has the
same settings and options. See the documentation there for more details.

The single rotor case is the same as in run_single_rotor.py, at the spin ratio of the two rotor
case. It is only simulated if it is not already stored by an earlier run of either script.

By default, all the angular positions from the experiments are simulated, with the recommended
variant of the model. As the simulations take time, a few angular positions can be chosen instead:

    python run_two_rotors.py --angular-positions 0 30 90
    python run_two_rotors.py --variant all

The force history of each case is stored as a time series in time_series_two_rotors_openfoam, and
the results are plotted with plot_results.py:

    python plot_results.py interference
'''

import argparse
import sys
from pathlib import Path

import pandas as pd

sys.path.append(str(
    Path(__file__).resolve().parents[5] / "pystormflow" / "examples" / "rotor_sails" /
    "deybach_wind_tunnel"
))

import rotor_setup
from plotting import (
    Solver,
    experimental_angular_positions,
    mean_coefficients,
    time_series_path,
    two_rotors_time_series_path,
    write_time_series,
)

from run_single_rotor import case_folder, run_case, simulate_force_history

if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--angular-positions", type=float, nargs="+", default=None,
        help="Angular positions of the interfering rotor, in degrees. All the angular positions "
        "from the experiments are simulated by default."
    )
    rotor_setup.add_variant_argument(parser, "simulate")
    parser.add_argument(
        "--nr-processes", type=int, default=None,
        help="Number of processes for the solver. The default is the number of physical cores."
    )

    args = parser.parse_args()

    if args.angular_positions is None:
        angular_positions = experimental_angular_positions()
    else:
        angular_positions = args.angular_positions

    spin_ratio = rotor_setup.TWO_ROTORS_SPIN_RATIO

    for variant in rotor_setup.variants_from_arguments(args.variant):
        single_path = time_series_path(variant, spin_ratio, Solver.OpenFOAM)

        if single_path.exists():
            print(f"{variant.label}, single rotor: using stored results")

            single_coefficients = mean_coefficients(pd.read_csv(single_path))
        else:
            print(f"{variant.label}, single rotor")

            single_coefficients = run_case(spin_ratio, variant, args.nr_processes)

        print(
            f"cl = {single_coefficients['cl']:.3f}, cd = {single_coefficients['cd']:.3f}",
            flush=True
        )

        for angular_position in angular_positions:
            print(f"{variant.label}, angular position of the interfering rotor: {angular_position}")

            file_path = two_rotors_time_series_path(variant, angular_position, Solver.OpenFOAM)

            force_history = simulate_force_history(
                case_folder(file_path),
                spin_ratio,
                variant,
                args.nr_processes,
                rotor_locations=rotor_setup.two_rotors_locations(angular_position)
            )

            write_time_series(force_history, file_path)

            coefficients = mean_coefficients(force_history)

            print(
                f"CL interference = {coefficients['cl'] / single_coefficients['cl']:.3f}, "
                f"CD interference = {coefficients['cd'] / single_coefficients['cd']:.3f}",
                flush=True
            )
