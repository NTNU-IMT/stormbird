"""
Copyright (C) 2026, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

Running actuator line simulations with either OpenFOAM or Stormflow, and returning the results in
the same format for both solvers, as ActuatorLineResults:

- forces: the integrated forces and moments on each wing as time series, with the same columns as
  the force file from the OpenFOAM interface ("time", "force_0.x", ..., "moment_0.x", ...).
- full_results: the full simulation results at some time steps, as dictionaries.

For the steady OpenFOAM simulations, the time is the iteration number. For Stormflow, it is the
physical time.

The functions require the solver to be available: OpenFOAM must be loaded in the terminal for the
OpenFOAM simulations, and pystormflow must be installed for the Stormflow simulations. tqdm is used
for a progress bar in the Stormflow simulations if it is installed.
"""

import json
import shutil
import tempfile
from pathlib import Path
from typing import Any, Callable

from ..openfoam.results import ActuatorLineResults
from ..openfoam.simulation_builder import SimulationBuilder as OpenFoamSimulationBuilder
from ..stormflow.simulation_builder import SimulationBuilder as StormflowSimulationBuilder

ACTUATOR_LINE_FORCE_DENSITY = 1.0
"""
The density used for the forces from the actuator line simulations. Both OpenFOAM (the
incompressible solvers, through cpp_actuator_line) and Stormflow solve the kinematic equations, and
set the density of the line force model to 1.0, regardless of the value in the setup. The forces in
the results are therefore forces per unit density, F / rho, and must be made non-dimensional with
this density, or multiplied by the real air density to get the physical forces.
"""


def run_openfoam_simulation(
    simulation_builder: OpenFoamSimulationBuilder,
    case_path: Path | str,
    nr_processes: int | None = None
) -> ActuatorLineResults:
    '''
    Writes, runs and reads an OpenFOAM case. An existing case folder at the same path is deleted
    first. The solver runs in parallel on the given number of processes, or on all physical cores
    if not given (see SimulationBuilder.run_foam_case). The simulation builder is stored in the
    results.
    '''
    case_path = Path(case_path)

    if case_path.exists():
        shutil.rmtree(case_path)

    simulation_builder.write_foam_case(case_path)
    simulation_builder.run_foam_case(case_path, nr_processes)

    results = simulation_builder.read_results(case_path)

    results.simulation_builder = simulation_builder.to_dict()

    return results


def full_result_as_dict(result: Any) -> dict[str, Any]:
    '''
    Converts a simulation result from pystormflow to the same dictionary as the full results from
    the OpenFOAM interface, by writing it to a temporary JSON file.
    '''
    with tempfile.TemporaryDirectory() as folder:
        file_path = Path(folder) / "full_result.json"

        result.write_to_file(str(file_path))

        full_result: dict[str, Any] = json.loads(file_path.read_text())

        return full_result


class _NoProgress:
    '''Used instead of a tqdm progress bar when tqdm is not installed'''
    def __init__(self, total: float) -> None:
        self.total = total
        self.n = 0.0

    def update(self, value: float) -> None:
        self.n += value

    def set_postfix_str(self, text: str, refresh: bool = True) -> None:
        pass

    def close(self) -> None:
        pass


def _progress_bar(end_time: float) -> Any:
    try:
        from tqdm import tqdm # type: ignore[import-not-found, import-untyped, unused-ignore]
    except ImportError:
        return _NoProgress(end_time)

    return tqdm(
        total=end_time,
        bar_format=(
            "{l_bar}{bar}| {n:.1f}/{total:.1f} s "
            "[elapsed: {elapsed}, remaining: {remaining}{postfix}]"
        ),
    )


def run_stormflow_simulation(
    simulation_builder: StormflowSimulationBuilder,
    end_time: float,
    courant_number: float,
    full_result_interval: float,
    progress_postfix: Callable[[Any], str] | None = None,
) -> ActuatorLineResults:
    '''
    Runs a Stormflow simulation until the end time, with a time step from the Courant number, and
    returns the results in the same format as for OpenFOAM. The forces are stored at every time
    step, and the full results at the given time interval, with the time step index as key. The
    last time step is always stored, and so is the simulation builder.

    A progress bar shows the simulated time and the estimated remaining time, if tqdm is installed.
    If given, progress_postfix is called with the latest pystormflow simulation result every 10
    time steps, and the returned text is shown after the progress bar, e.g., to show the current
    forces.
    '''
    from pystormflow import Simulation # type: ignore[import-not-found, import-untyped, unused-ignore]

    simulation = Simulation(simulation_builder.to_json_string())

    forces: dict[str, list[float]] = {"time": []}
    full_results: dict[int, dict[str, Any]] = {}

    time = 0.0
    step_index = 0
    next_full_result_time = full_result_interval
    result = None

    progress = _progress_bar(end_time)

    while time < end_time:
        time_step = simulation.time_step_from_courant_number(courant_number)

        simulation.do_step(time, time_step)

        time += time_step
        step_index += 1

        progress.update(min(time_step, progress.total - progress.n))

        result = simulation.get_stormbird_simulation_result()

        if result is None:
            continue

        forces["time"].append(time)

        for wing_index, (force, moment) in enumerate(
            zip(result.integrated_forces, result.integrated_moments)
        ):
            for name, values in (
                (f"force_{wing_index}", force.total), (f"moment_{wing_index}", moment.total)
            ):
                for component, value in zip("xyz", values):
                    forces.setdefault(f"{name}.{component}", []).append(value)

        if time >= next_full_result_time:
            full_results[step_index] = full_result_as_dict(result)

            next_full_result_time += full_result_interval

        if progress_postfix is not None and step_index % 10 == 0:
            progress.set_postfix_str(progress_postfix(result), refresh=False)

    progress.close()

    if result is not None and step_index not in full_results:
        full_results[step_index] = full_result_as_dict(result)

    return ActuatorLineResults(
        forces=forces,
        full_results=full_results,
        simulation_builder=simulation_builder.to_dict(),
    )
