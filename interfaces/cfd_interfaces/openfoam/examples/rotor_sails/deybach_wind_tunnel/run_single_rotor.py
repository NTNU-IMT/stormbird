'''
Runs actuator line simulations of a single rotor sail with OpenFOAM, for a range of spin ratios.
This is the OpenFOAM version of run_single_rotor.py in
interfaces/pystormflow/examples/rotor_sails/deybach_wind_tunnel, which is the reference for the
setup. See the README there for a description of the cases and the model variants. By default, only
the recommended variant of the model is simulated. Use --variant to choose one or more other
variants, or all of them:

    python run_single_rotor.py --variant lifting_line no_corrections
    python run_single_rotor.py --variant all

Everything except the mesh is imported from the Stormflow example: the actuator line model, the
domain (the wind tunnel), the flow settings and the post-processing of the forces are imported from
rotor_setup.py, and the results are stored and plotted with plotting.py, in the same way as for
Stormflow. The force history of each case is stored as a time series in time_series_openfoam, and
the results are plotted with plot_results.py:

    python plot_results.py means

The only difference is the mesh. Stormflow uses a uniform grid, while the OpenFOAM mesh has a
background mesh that is refined in a cylinder around each rotor. The resolution in the refinement
zone is the same as the uniform grid in Stormflow, while the background mesh is coarser by the
factor BACKGROUND_COARSENING_FACTOR.

The case settings that are not exposed in the OpenFOAM simulation builder are changed directly in
the case files, so that the flow settings match Stormflow: the turbulence model is turned off, and
the viscosity is set to the same constant value as in Stormflow.
'''

import argparse
import os
import shutil
import sys
from pathlib import Path

import pandas as pd
from stormbird_setup.openfoam import (
    BoundaryConditionType,
    DynamicTimeControl,
    GridBuilder,
    RefinementBuilder,
    SimulationBuilder,
    read_foam_file,
    write_foam_file,
)
from stormbird_setup.spatial_vector import SpatialVector

# The shared setup is imported from the Stormflow example. The folder is added to the end of the
# path, so that the scripts in this folder are used over the scripts with the same name there.
sys.path.append(str(
    Path(__file__).resolve().parents[5] / "pystormflow" / "examples" / "rotor_sails" /
    "deybach_wind_tunnel"
))

import rotor_setup
from plotting import (
    Solver,
    mean_coefficients,
    time_series_path,
    write_time_series,
)
from rotor_setup import Variant

# The resolution of the background mesh is the resolution around the rotors divided by this factor
BACKGROUND_COARSENING_FACTOR = 2

# Size of the refinement zone around each rotor, as a factor on the width of the projection
# function in the actuator line model. See RefinementBuilder for the details. With the current
# projection settings, the radius of the zone is about one diameter.
REFINEMENT_PADDING = 4.0

# The initial and maximum time step. The time step is otherwise set by the Courant number, as in
# Stormflow.
MAX_TIME_STEP = 0.05

def case_folder(time_series_file_path: Path) -> Path:
    '''
    The folder of the OpenFOAM case that produces the given time series. The case folders are
    organized in the same way as the time series, so that it is easy to find the case for a result.
    OpenFOAM must be loaded in the terminal.
    '''
    return (
        Path(os.environ['FOAM_RUN']) / 'stormbird_rotor_sails' /
        time_series_file_path.parent.name / time_series_file_path.stem
    )

def simulation_builder(
    spin_ratio: float,
    variant: Variant,
    cells_per_chord_length: int = rotor_setup.CELLS_PER_CHORD_LENGTH,
    rotor_locations: list[SpatialVector] | None = None
) -> SimulationBuilder:
    '''
    The same case as in the Stormflow version, except for the mesh. The domain is the wind tunnel
    from the experiments, with slip walls on the sides and the top.
    '''
    if cells_per_chord_length % BACKGROUND_COARSENING_FACTOR != 0:
        raise ValueError(
            "The number of cells per chord length must be divisible by "
            f"{BACKGROUND_COARSENING_FACTOR}, so that the resolution around the rotors is the same "
            f"as in Stormflow. Got {cells_per_chord_length}"
        )

    return SimulationBuilder(
        grid=GridBuilder(
            start_point=rotor_setup.tunnel_start_point(),
            end_point=rotor_setup.tunnel_end_point(),
            background_mesh_cells_per_representative_length=(
                cells_per_chord_length // BACKGROUND_COARSENING_FACTOR
            ),
            actuator_line_refinement=RefinementBuilder(
                cells_per_representative_length=cells_per_chord_length,
                padding=REFINEMENT_PADDING
            ),
        ),
        wind_condition=rotor_setup.wind_condition(),
        linear_velocity=SpatialVector(),
        actuator_line=rotor_setup.actuator_line_builder(spin_ratio, variant, rotor_locations),
        time_control=DynamicTimeControl(
            time_step=MAX_TIME_STEP,
            end_time=rotor_setup.END_TIME,
            max_courant_number=rotor_setup.COURANT_NUMBER
        ),
        boundary_condition_type=BoundaryConditionType.WindTunnel,
    )

def write_case(builder: SimulationBuilder, path: Path) -> None:
    '''
    Writes the case, and changes the flow settings to match Stormflow: no turbulence model, and a
    constant viscosity
    '''
    builder.write_foam_case(path)

    turbulence_properties_path = path / "constant" / "turbulenceProperties"

    turbulence_properties = read_foam_file(turbulence_properties_path)
    turbulence_properties["simulationType"] = "laminar"
    turbulence_properties.pop("RAS", None)

    write_foam_file(turbulence_properties_path, turbulence_properties)

    transport_properties_path = path / "constant" / "transportProperties"

    transport_properties = read_foam_file(transport_properties_path)
    transport_properties["nu"] = rotor_setup.EFFECTIVE_VISCOSITY

    write_foam_file(transport_properties_path, transport_properties)

def run_case(spin_ratio: float, variant: Variant, nr_processes: int | None) -> dict[str, float]:
    file_path = time_series_path(variant, spin_ratio, Solver.OpenFOAM)

    force_history = simulate_force_history(
        case_folder(file_path), spin_ratio, variant, nr_processes
    )

    write_time_series(force_history, file_path)

    return mean_coefficients(force_history)

def simulate_force_history(
    path: Path,
    spin_ratio: float,
    variant: Variant,
    nr_processes: int | None,
    cells_per_chord_length: int = rotor_setup.CELLS_PER_CHORD_LENGTH,
    rotor_locations: list[SpatialVector] | None = None
) -> pd.DataFrame:
    '''
    Writes and runs a single case in the given folder, and returns the force on the first rotor as
    a function of time, in the same format as in the Stormflow version. An existing case in the
    folder is replaced. A single rotor at the origin is used if no rotor locations are given.
    '''
    if path.exists():
        shutil.rmtree(path)

    builder = simulation_builder(spin_ratio, variant, cells_per_chord_length, rotor_locations)

    write_case(builder, path)

    builder.run_foam_case(path, nr_processes=nr_processes)

    forces = builder.read_results(path).forces

    return pd.DataFrame({
        "time": forces["time"],
        "force.x": forces["force_0.x"],
        "force.y": forces["force_0.y"],
    })


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--spin-ratios", type=float, nargs="+", default=rotor_setup.SPIN_RATIOS,
        help="Spin ratios to simulate"
    )
    parser.add_argument(
        "--nr-processes", type=int, default=None,
        help="Number of processes for the solver. The default is the number of physical cores."
    )
    rotor_setup.add_variant_argument(parser, "simulate")

    args = parser.parse_args()

    for variant in rotor_setup.variants_from_arguments(args.variant):
        for spin_ratio in args.spin_ratios:
            print(f"{variant.label}, spin ratio: {spin_ratio}")

            coefficients = run_case(spin_ratio, variant, args.nr_processes)

            print(f"cl = {coefficients['cl']:.3f}, cd = {coefficients['cd']:.3f}", flush=True)
