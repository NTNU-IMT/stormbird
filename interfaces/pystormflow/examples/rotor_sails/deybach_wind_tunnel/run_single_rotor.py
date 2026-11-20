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

For variants with a lifting line correction with panels, i.e., the dynamic wake, the shape of the
wake can be exported at a regular interval, given in seconds:

    python run_single_rotor.py --variant projected_tip_loss_and_dynamic_lifting_line --export-wake-interval 2.0

The files are stored in a folder for each case in correction_wake_single_rotor_stormflow, together
with a correction_wake.vtk.series file, which opens all the files as one time series in ParaView.
'''

import argparse
import json
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

def wake_folder(variant: Variant, spin_ratio: float) -> Path:
    '''
    The folder for the exported wake of the lifting line correction in a single rotor case, with
    the same name as the time series.
    '''
    return (
        Solver.Stormflow.folder / f"correction_wake_single_rotor_{Solver.Stormflow.value}" /
        time_series_path(variant, spin_ratio, Solver.Stormflow).stem
    )

def add_vtk_arguments(parser: argparse.ArgumentParser) -> None:
    parser.add_argument(
        "--export-vtk", action="store_true",
        help="Export the flow fields at the end of each case as VTK files"
    )
    parser.add_argument(
        "--export-wake-interval", type=float, default=None,
        help="Export the shape of the wake in the lifting line correction with this interval, in "
        "seconds. Only for variants with a wake that has panels, i.e., the dynamic wake."
    )

class WakeExporter:
    '''
    Exports the wake of the lifting line correction at a regular interval, to numbered VTK files in
    a folder, together with a ParaView series file with the time of each file.
    '''
    def __init__(self, folder: Path, interval: float):
        self.folder = folder
        self.interval = interval
        self.next_time = interval
        self.files: list[dict] = []

    def export_if_time(self, simulation: Simulation, time: float) -> None:
        if time < self.next_time:
            return

        self.next_time += self.interval

        self.folder.mkdir(parents=True, exist_ok=True)

        file_name = f"correction_wake_{len(self.files):04d}.vtk"

        if simulation.export_correction_wake_as_vtk(str(self.folder / file_name), binary=True):
            self.files.append({"name": file_name, "time": time})

            series = {"file-series-version": "1.0", "files": self.files}

            (self.folder / "correction_wake.vtk.series").write_text(json.dumps(series, indent=2))


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
    export_vtk: bool = False,
    export_wake_interval: float | None = None,
) -> dict[str, float]:
    force_history = simulate_force_history(
        spin_ratio,
        variant,
        compute_platform,
        vtk_export_path=vtk_path(variant, spin_ratio) if export_vtk else None,
        wake_exporter=(
            WakeExporter(wake_folder(variant, spin_ratio), export_wake_interval)
            if export_wake_interval is not None else None
        ),
    )

    write_time_series(force_history, time_series_path(variant, spin_ratio, Solver.Stormflow))

    return mean_coefficients(force_history)

def simulate_force_history(
    spin_ratio: float,
    variant: Variant,
    compute_platform: ComputePlatform,
    cells_per_chord_length: int = rotor_setup.CELLS_PER_CHORD_LENGTH,
    rotor_locations: list[SpatialVector] | None = None,
    vtk_export_path: Path | None = None,
    wake_exporter: WakeExporter | None = None,
    end_time: float = rotor_setup.END_TIME,
) -> pd.DataFrame:
    '''
    Runs a single case, and returns the force on the first rotor as a function of time. A single
    rotor at the origin is used if no rotor locations are given. The flow fields at the end of the
    simulation are exported as a VTK file to the given path, if any, and the wake of the lifting
    line correction with the wake exporter, if any.
    '''
    builder = simulation_builder(
        spin_ratio, variant, compute_platform, cells_per_chord_length, rotor_locations
    )

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

            if wake_exporter is not None:
                wake_exporter.export_if_time(simulation, time)

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

            coefficients = run_case(
                spin_ratio, variant, compute_platform, args.export_vtk, args.export_wake_interval
            )

            print(f"cl = {coefficients['cl']:.3f}, cd = {coefficients['cd']:.3f}")
