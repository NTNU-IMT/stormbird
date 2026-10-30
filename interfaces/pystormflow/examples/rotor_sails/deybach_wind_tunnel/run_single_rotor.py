'''
Runs actuator line simulations of a single rotor sail with Stormflow, for a range of spin ratios. See
rotor_setup.py for the actuator line model and the flow settings, which are shared with the OpenFOAM
version of this example. By default, only the recommended variant of the model is simulated. Use
--variant to choose one or more other variants, or all of them:

    python run_single_rotor.py --variant lifting_line no_corrections
    python run_single_rotor.py --variant all

The force history of each case is stored as a time series in time_series_single_rotor_stormflow, and
the results are plotted with plotting.py:

    python plotting.py means

The flow fields at the end of each case can also be exported as VTK files, which can be viewed in
for instance ParaView:

    python run_single_rotor.py --export-vtk

The VTK files are stored in vtk_single_rotor_stormflow, with the same name as the time series.
'''

import argparse
from pathlib import Path

import pandas as pd
from pystormflow import Simulation
from stormbird_setup.spatial_vector import SpatialVector
from stormbird_setup.stormflow import (
    BoundaryConditionType,
    ComputePlatform,
    GridBuilder,
    MultigridSettingsBuilder,
    PressureSolverBuilder,
    SimulationBuilder,
    VelocitySolverBuilder,
    TurbulenceSolverBuilder
)
from tqdm import tqdm

import rotor_setup
from plotting import Solver, mean_coefficients, time_series_path, write_time_series
from rotor_setup import Variant

def vtk_path(variant: Variant, spin_ratio: float) -> Path:
    '''
    The path of the VTK file of a single rotor case. The name of the file is the same as the name of
    the time series.
    '''
    return (
        Solver.Stormflow.folder / f"vtk_single_rotor_{Solver.Stormflow.value}" /
        time_series_path(variant, spin_ratio, Solver.Stormflow).with_suffix(".vtk").name
    )

def add_vtk_arguments(parser: argparse.ArgumentParser) -> None:
    parser.add_argument(
        "--export-vtk", action="store_true",
        help="Export the flow fields at the end of each case as VTK files"
    )


def simulation_builder(
    spin_ratio: float,
    variant: Variant,
    compute_platform: ComputePlatform,
    cells_per_chord_length: int = rotor_setup.CELLS_PER_CHORD_LENGTH,
    rotor_locations: list[SpatialVector] | None = None
) -> SimulationBuilder:
    '''
    The domain is the wind tunnel from the experiments, with slip walls on the sides and the top
    '''
    return SimulationBuilder(
        grid=GridBuilder(
            start_point=rotor_setup.tunnel_start_point(),
            end_point=rotor_setup.tunnel_end_point(),
            cells_per_representative_length=(cells_per_chord_length,) * 3,
        ),
        wind_condition=rotor_setup.wind_condition(),
        linear_velocity=SpatialVector(),
        actuator_line=rotor_setup.actuator_line_builder(spin_ratio, variant, rotor_locations),
        effective_viscosity=rotor_setup.EFFECTIVE_VISCOSITY,
        velocity_solver=VelocitySolverBuilder(
            compute_platform=compute_platform,
        ),
        pressure_solver=PressureSolverBuilder(
            settings=MultigridSettingsBuilder(compute_platform=compute_platform)
        ),
        boundary_condition_type=BoundaryConditionType.WindTunnel,
        turbulence = TurbulenceSolverBuilder()
    )

def run_case(
    spin_ratio: float,
    variant: Variant,
    compute_platform: ComputePlatform,
    export_vtk: bool = False
) -> dict[str, float]:
    force_history = simulate_force_history(
        spin_ratio,
        variant,
        compute_platform,
        vtk_export_path=vtk_path(variant, spin_ratio) if export_vtk else None
    )

    write_time_series(force_history, time_series_path(variant, spin_ratio, Solver.Stormflow))

    return mean_coefficients(force_history)

def simulate_force_history(
    spin_ratio: float,
    variant: Variant,
    compute_platform: ComputePlatform,
    cells_per_chord_length: int = rotor_setup.CELLS_PER_CHORD_LENGTH,
    rotor_locations: list[SpatialVector] | None = None,
    vtk_export_path: Path | None = None
) -> pd.DataFrame:
    '''
    Runs a single case, and returns the force on the first rotor as a function of time. A single
    rotor at the origin is used if no rotor locations are given. The flow fields at the end of the
    simulation are exported as a VTK file to the given path, if any.
    '''
    builder = simulation_builder(
        spin_ratio, variant, compute_platform, cells_per_chord_length, rotor_locations
    )

    end_time = rotor_setup.END_TIME

    simulation = Simulation(builder.to_json_string())

    time_history: list[float] = []
    force_x: list[float] = []
    force_y: list[float] = []

    time = 0.0

    with tqdm(
        total=end_time,
        bar_format="{l_bar}{bar}| {n:.1f}/{total:.1f} s [elapsed: {elapsed}, remaining: {remaining}]") as progress:
        while time < end_time:
            time_step = simulation.time_step_from_courant_number(rotor_setup.COURANT_NUMBER)

            simulation.do_step(time, time_step)

            time += time_step

            result = simulation.get_stormbird_simulation_result()

            if result is not None:
                force = result.integrated_forces[0].total

                time_history.append(time)
                force_x.append(force[0])
                force_y.append(force[1])

            progress.update(min(time_step, progress.total - progress.n))

    if vtk_export_path is not None:
        vtk_export_path.parent.mkdir(exist_ok=True)

        simulation.export_fields_as_vtk(str(vtk_export_path), binary=True)

    return pd.DataFrame({"time": time_history, "force.x": force_x, "force.y": force_y})


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--spin-ratios", type=float, nargs="+", default=rotor_setup.SPIN_RATIOS,
        help="Spin ratios to simulate"
    )
    parser.add_argument("--cpu", action="store_true", help="Run on the CPU instead of the GPU")
    rotor_setup.add_variant_argument(parser, "simulate")
    add_vtk_arguments(parser)

    args = parser.parse_args()

    compute_platform = ComputePlatform.CPU if args.cpu else ComputePlatform.GPU

    for variant in rotor_setup.variants_from_arguments(args.variant):
        for spin_ratio in args.spin_ratios:
            print(f"{variant.label}, spin ratio: {spin_ratio}")

            coefficients = run_case(spin_ratio, variant, compute_platform, args.export_vtk)

            print(f"cl = {coefficients['cl']:.3f}, cd = {coefficients['cd']:.3f}")
