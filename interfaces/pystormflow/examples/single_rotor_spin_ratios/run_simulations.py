'''
Runs actuator line simulations of a single rotor sail with Stormflow, for a range of spin ratios and 
compares the results against the data from Deybach et al. (2024). See rotor_setup.py for the 
actuator line model, which is shared with the OpenFOAM version of this example.
'''

import argparse

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
)
from tqdm import tqdm

import rotor_setup
from plotting import TIME_SERIES_FOLDER, mean_coefficients, time_series_path
from rotor_setup import Variant

COURANT_NUMBER = 0.75

def simulation_builder(
    spin_ratio: float,
    variant: Variant,
    compute_platform: ComputePlatform,
    cells_per_chord_length: int = rotor_setup.CELLS_PER_CHORD_LENGTH
) -> SimulationBuilder:
    return SimulationBuilder(
        grid=GridBuilder(
            start_point=SpatialVector(x=-40.0, y=-1.04*rotor_setup.HEIGHT, z=0.0),
            end_point=SpatialVector(x=80.0, y=1.04*rotor_setup.HEIGHT, z=rotor_setup.HEIGHT*1.25),
            cells_per_representative_length=(cells_per_chord_length,) * 3,
        ),
        wind_condition=rotor_setup.wind_condition(),
        linear_velocity=SpatialVector(),
        actuator_line=rotor_setup.actuator_line_builder(spin_ratio, variant),
        velocity_solver=VelocitySolverBuilder(compute_platform=compute_platform),
        pressure_solver=PressureSolverBuilder(
            settings=MultigridSettingsBuilder(compute_platform=compute_platform)
        ),
        boundary_condition_type=BoundaryConditionType.WindTunnel,
    )

def run_case(spin_ratio: float, variant: Variant, compute_platform: ComputePlatform) -> dict[str, float]:
    force_history = simulate_force_history(spin_ratio, variant, compute_platform)

    # The full force history is stored, and the mean values are calculated from it when plotting
    TIME_SERIES_FOLDER.mkdir(exist_ok=True)

    force_history.to_csv(time_series_path(variant, spin_ratio), index=False)

    return mean_coefficients(force_history)

def simulate_force_history(
    spin_ratio: float,
    variant: Variant,
    compute_platform: ComputePlatform,
    cells_per_chord_length: int = rotor_setup.CELLS_PER_CHORD_LENGTH
) -> pd.DataFrame:
    '''
    Runs a single case, and returns the force on the rotor as a function of time
    '''
    builder = simulation_builder(spin_ratio, variant, compute_platform, cells_per_chord_length)

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
            time_step = simulation.time_step_from_courant_number(COURANT_NUMBER)

            simulation.do_step(time, time_step)

            time += time_step

            result = simulation.get_stormbird_simulation_result()

            if result is not None:
                force = result.integrated_forces[0].total

                time_history.append(time)
                force_x.append(force[0])
                force_y.append(force[1])

            progress.update(min(time_step, progress.total - progress.n))

    return pd.DataFrame({"time": time_history, "force.x": force_x, "force.y": force_y})


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--spin-ratios", type=float, nargs="+", default=rotor_setup.SPIN_RATIOS,
        help="Spin ratios to simulate"
    )
    parser.add_argument("--cpu", action="store_true", help="Run on the CPU instead of the GPU")

    args = parser.parse_args()

    compute_platform = ComputePlatform.CPU if args.cpu else ComputePlatform.GPU

    variant = Variant.TipLossCorrection

    for spin_ratio in args.spin_ratios:
        print(f"{variant.label}, spin ratio: {spin_ratio}")

        coefficients = run_case(spin_ratio, variant, compute_platform)

        print(f"cl = {coefficients['cl']:.3f}, cd = {coefficients['cd']:.3f}")
