'''
Runs actuator line simulations of two rotor sails with Stormflow, where the interfering rotor is
placed at different angular positions around the rotor at the origin. This is the actuator line
version of two_rotors_interference_factors.py in the lifting line example in
interfaces/pystormbird/examples/rotor_sails/deybach_wind_tunnel, and the results are compared
against the same experiments by Deybach et al. (2026).

The interference factors are the lift and drag on the rotor at the origin divided by the lift and
drag on the same rotor when it stands alone. The single rotor case is the same as in
run_single_rotor.py, at the spin ratio of the two rotor case. It is only simulated if it is not
already stored by an earlier run of either script.

By default, all the angular positions from the experiments are simulated, with the recommended
variant of the model. As the simulations take time, a few angular positions can be chosen instead:

    python run_two_rotors.py --angular-positions 0 30 90
    python run_two_rotors.py --variant all

The force history of each case is stored as a time series in time_series_two_rotors_stormflow, and
the results are plotted with plotting.py:

    python plotting.py interference

The flow fields at the end of each case can also be exported as VTK files with --export-vtk. The VTK
files are stored in vtk_two_rotors_stormflow, with the same name as the time series. The VTK file of
the single rotor case is only exported if the single rotor case is simulated, and is then stored in
vtk_single_rotor_stormflow.
'''

import argparse
from pathlib import Path

import pandas as pd
from stormbird_setup.stormflow import ComputePlatform

import rotor_setup
from plotting import (
    Solver,
    experimental_angular_positions,
    mean_coefficients,
    time_series_path,
    two_rotors_time_series_path,
    write_time_series,
)
from rotor_setup import Variant
from run_single_rotor import add_vtk_arguments, run_case, simulate_force_history


def two_rotors_vtk_path(variant: Variant, angular_position: float) -> Path:
    '''
    The path of the VTK file of a two rotor case. The name of the file is the same as the name of
    the time series.
    '''
    return (
        Solver.Stormflow.folder / f"vtk_two_rotors_{Solver.Stormflow.value}" /
        two_rotors_time_series_path(
            variant, angular_position, Solver.Stormflow
        ).with_suffix(".vtk").name
    )


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--angular-positions", type=float, nargs="+", default=None,
        help="Angular positions of the interfering rotor, in degrees. All the angular positions "
        "from the experiments are simulated by default."
    )
    rotor_setup.add_variant_argument(parser, "simulate")
    parser.add_argument("--cpu", action="store_true", help="Run on the CPU instead of the GPU")
    add_vtk_arguments(parser)

    args = parser.parse_args()

    compute_platform = ComputePlatform.CPU if args.cpu else ComputePlatform.GPU

    if args.angular_positions is None:
        angular_positions = experimental_angular_positions()
    else:
        angular_positions = args.angular_positions

    spin_ratio = rotor_setup.TWO_ROTORS_SPIN_RATIO

    for variant in rotor_setup.variants_from_arguments(args.variant):
        single_path = time_series_path(variant, spin_ratio, Solver.Stormflow)

        if single_path.exists():
            print(f"{variant.label}, single rotor: using stored results")

            single_coefficients = mean_coefficients(pd.read_csv(single_path))
        else:
            print(f"{variant.label}, single rotor")

            single_coefficients = run_case(spin_ratio, variant, compute_platform, args.export_vtk)

        print(
            f"cl = {single_coefficients['cl']:.3f}, cd = {single_coefficients['cd']:.3f}",
            flush=True
        )

        for angular_position in angular_positions:
            print(f"{variant.label}, angular position of the interfering rotor: {angular_position}")

            force_history = simulate_force_history(
                spin_ratio,
                variant,
                compute_platform,
                rotor_locations=rotor_setup.two_rotors_locations(angular_position),
                vtk_export_path=(
                    two_rotors_vtk_path(variant, angular_position) if args.export_vtk else None
                )
            )

            write_time_series(
                force_history,
                two_rotors_time_series_path(variant, angular_position, Solver.Stormflow)
            )

            coefficients = mean_coefficients(force_history)

            print(
                f"CL interference = {coefficients['cl'] / single_coefficients['cl']:.3f}, "
                f"CD interference = {coefficients['cd'] / single_coefficients['cd']:.3f}",
                flush=True
            )
