'''
Mesh convergence test for the OpenFOAM simulations in run_simulations.py. A single spin ratio is run
for all variants of the actuator line model, with increasing resolution at the rotor, starting with
the lowest. The results are plotted every time a case is finished. The Stormflow version of this
example has the same test.
'''

import os
import shutil
import argparse
from pathlib import Path

from run_simulations import simulation_builder, rotor_setup, Variant

RESULTS_PATH = Path(__file__).parent / "mesh_convergence_openfoam.json"


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
    parser.add_argument("--nr-processes", type=int, default=None, help="Number of processes for the solver")
    parser.add_argument("--plot-only", action="store_true", help="Only plot the stored results")

    args = parser.parse_args()

    plot_path = Path(__file__).parent / "mesh_convergence_openfoam.png"

    # Results from earlier runs are kept, so that individual cases can be rerun
    results = rotor_setup.read_results(RESULTS_PATH)

    if not args.plot_only:
        run_folder = (
            Path(os.environ['FOAM_RUN']) / 'stormbird_single_rotor_spin_ratios' / 'mesh_convergence'
        )

        for cells_per_chord_length in sorted(args.cells_per_chord_length):
            for variant in map(Variant, args.variants):
                print(f"{variant.label}, cells per chord length: {cells_per_chord_length}")

                case_folder = (
                    run_folder / f"{variant.value}_spin_ratio_{args.spin_ratio}_cells_{cells_per_chord_length}"
                )

                if case_folder.exists():
                    shutil.rmtree(case_folder)

                builder = simulation_builder(args.spin_ratio, variant, cells_per_chord_length)

                builder.write_foam_case(case_folder)
                builder.run_foam_case(case_folder, nr_processes=args.nr_processes)

                forces = builder.read_results(case_folder).forces

                coefficients = rotor_setup.force_coefficients(
                    forces['time'], forces['force_0.x'], forces['force_0.y']
                )

                print(f"cl = {coefficients['cl']:.3f}, cd = {coefficients['cd']:.3f}", flush=True)

                results.setdefault(variant.value, {})[str(cells_per_chord_length)] = coefficients

                rotor_setup.write_results(results, RESULTS_PATH)
                rotor_setup.plot_mesh_convergence(results, args.spin_ratio, "OpenFOAM", plot_path)

    rotor_setup.plot_mesh_convergence(results, args.spin_ratio, "OpenFOAM", plot_path)
