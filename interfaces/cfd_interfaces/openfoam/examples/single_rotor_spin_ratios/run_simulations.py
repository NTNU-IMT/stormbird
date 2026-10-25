'''
Runs actuator line simulations of a single rotor sail with OpenFOAM, for a range of spin ratios,
both with and without the lifting line correction, and compares the results against the data from
Deybach et al. (2024).

The actuator line model is imported from the Stormflow version of this example, in
interfaces/pystormflow/examples/single_rotor_spin_ratios/rotor_setup.py, so that both solvers use
exactly the same settings. Only the mesh is different: the domain is larger, with a coarse background
mesh, but the resolution around the rotor is the same as in Stormflow.
'''

import os
import sys
import shutil
import argparse
from pathlib import Path

from stormbird_setup.spatial_vector import SpatialVector
from stormbird_setup.openfoam import (
    SimulationBuilder, GridBuilder, RefinementBuilder, DynamicTimeControl
)

sys.path.append(
    str(Path(__file__).resolve().parents[4] / "pystormflow" / "examples" / "single_rotor_spin_ratios")
)

import rotor_setup
from rotor_setup import Variant

RESULTS_PATH = Path(__file__).parent / "results_openfoam.json"

def simulation_builder(
    spin_ratio: float,
    variant: Variant,
    cells_per_chord_length: int = rotor_setup.CELLS_PER_CHORD_LENGTH
) -> SimulationBuilder:
    return SimulationBuilder(
        grid=GridBuilder(
            start_point=SpatialVector(x=-150.0, y=-150.0, z=0.0),
            end_point=SpatialVector(x=300.0, y=150.0, z=150.0),
            background_mesh_cells_per_representative_length=1,
            actuator_line_refinement=RefinementBuilder(
                cells_per_representative_length=cells_per_chord_length, padding=1.0
            ),
            geometry_cells_per_representative_length=cells_per_chord_length,
        ),
        wind_condition=rotor_setup.wind_condition(),
        linear_velocity=SpatialVector(),
        actuator_line=rotor_setup.actuator_line_builder(spin_ratio, variant),
        slip_geometries=rotor_setup.slip_geometries(),
        time_control=DynamicTimeControl(
            time_step=0.05, end_time=rotor_setup.END_TIME, max_courant_number=1.0
        ),
    )


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--spin-ratios", type=float, nargs="+", default=rotor_setup.SPIN_RATIOS,
        help="Spin ratios to simulate"
    )
    parser.add_argument(
        "--variants", type=str, nargs="+", default=[variant.value for variant in Variant],
        choices=[variant.value for variant in Variant], help="Variants of the actuator line model to simulate"
    )
    parser.add_argument("--nr-processes", type=int, default=None, help="Number of processes for the solver")
    parser.add_argument("--plot-only", action="store_true", help="Only plot the stored results")

    args = parser.parse_args()

    # Results from earlier runs are kept, so that individual cases can be rerun
    results = rotor_setup.read_results(RESULTS_PATH)

    if not args.plot_only:
        run_folder = Path(os.environ['FOAM_RUN']) / 'stormbird_single_rotor_spin_ratios'

        for variant in map(Variant, args.variants):
            for spin_ratio in args.spin_ratios:
                print(f"{variant.label}, spin ratio: {spin_ratio}")

                case_folder = run_folder / variant.value / f"spin_ratio_{spin_ratio}"

                if case_folder.exists():
                    shutil.rmtree(case_folder)

                builder = simulation_builder(spin_ratio, variant)

                builder.write_foam_case(case_folder)
                builder.run_foam_case(case_folder, nr_processes=args.nr_processes)

                forces = builder.read_results(case_folder).forces

                coefficients = rotor_setup.force_coefficients(
                    forces['time'], forces['force_0.x'], forces['force_0.y']
                )

                print(f"cl = {coefficients['cl']:.3f}, cd = {coefficients['cd']:.3f}")

                results.setdefault(variant.value, {})[str(spin_ratio)] = coefficients

                rotor_setup.write_results(results, RESULTS_PATH)

    rotor_setup.plot_results(results, "OpenFOAM", Path(__file__).parent / "results_openfoam.png")
