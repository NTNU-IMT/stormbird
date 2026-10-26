'''
Storage and plotting of the results from run_single_rotor.py, mesh_convergence_test.py and
run_two_rotors.py. The force histories are stored as time series, and the mean values are calculated
from them when plotting. Run as a script to plot a single time series, the mean values of all stored
simulations against the data from Deybach et al. (2024), the results of the mesh convergence test,
or the interference factors for two rotors against the data from Deybach et al. (2026) and the
lifting line example:

    python plotting.py means
    python plotting.py time-series --spin-ratio 3.0 --variant tip_loss_and_lifting_line
    python plotting.py time-series --spin-ratio 3.0 --cells-per-chord-length 16
    python plotting.py time-series --angular-position 30
    python plotting.py mesh-convergence --spin-ratio 3.0 --variant all
    python plotting.py interference --variant all

The time series from the mesh convergence test are plotted when --cells-per-chord-length is given,
and the time series from the two rotor case when --angular-position is given. The variant is the
recommended one by default. The plots are shown directly by default. Use --save (before the command)
to save them to file instead.

The OpenFOAM version of these examples, in
interfaces/cfd_interfaces/openfoam/examples/rotor_sails/deybach_wind_tunnel, stores and plots the
results in the same way, through this module. The results from each solver are stored in the example
folder of that solver. Use --solver to choose which results to plot.

The lifting line results in the interference plot are computed when plotting, with the quasi-steady
lifting line model from interfaces/pystormbird/examples/rotor_sails/deybach_wind_tunnel. This
requires pystormbird.
'''

import argparse
import sys
from enum import Enum
from pathlib import Path

import matplotlib.pyplot as plt
import numpy as np
import pandas as pd

import rotor_setup
from rotor_setup import ASPECT_RATIO, Variant

INTERFACES_FOLDER = Path(__file__).resolve().parents[4]

LIFTING_LINE_EXAMPLE_FOLDER = (
    INTERFACES_FOLDER / "pystormbird" / "examples" / "rotor_sails" / "deybach_wind_tunnel"
)

COMPARISON_DATA_PATH = LIFTING_LINE_EXAMPLE_FOLDER / "data" / "deybach_2024_single_rotor.csv"
INTERFERENCE_DATA_PATH = (
    LIFTING_LINE_EXAMPLE_FOLDER / "data" / "deybach_2026_interference_factors.csv"
)

SPIN_RATIO_SEPARATOR = "_spin_ratio_"
CELLS_SEPARATOR = "_cells_"
ANGULAR_POSITION_SEPARATOR = "_angular_position_"

class Solver(Enum):
    '''
    The solvers that the examples are run with. The results from each solver are stored in the
    example folder of that solver.
    '''
    Stormflow = "stormflow"
    OpenFOAM = "openfoam"

    @property
    def label(self) -> str:
        match self:
            case Solver.Stormflow:
                return "Stormflow"
            case Solver.OpenFOAM:
                return "OpenFOAM"

    @property
    def folder(self) -> Path:
        match self:
            case Solver.Stormflow:
                return Path(__file__).resolve().parent
            case Solver.OpenFOAM:
                return (
                    INTERFACES_FOLDER / "cfd_interfaces" / "openfoam" / "examples" / "rotor_sails" /
                    "deybach_wind_tunnel"
                )

def time_series_folder(solver: Solver) -> Path:
    return solver.folder / f"time_series_{solver.value}"

def mesh_convergence_time_series_folder(solver: Solver) -> Path:
    return solver.folder / f"time_series_mesh_convergence_{solver.value}"

def two_rotors_time_series_folder(solver: Solver) -> Path:
    return solver.folder / f"time_series_two_rotors_{solver.value}"

def time_series_path(variant: Variant, spin_ratio: float, solver: Solver) -> Path:
    return time_series_folder(solver) / (
        f"{variant.value}{SPIN_RATIO_SEPARATOR}{float(spin_ratio)}.csv"
    )

def mesh_convergence_time_series_path(
    variant: Variant,
    spin_ratio: float,
    cells_per_chord_length: int,
    solver: Solver
) -> Path:
    return mesh_convergence_time_series_folder(solver) / (
        f"{variant.value}{SPIN_RATIO_SEPARATOR}{float(spin_ratio)}"
        f"{CELLS_SEPARATOR}{cells_per_chord_length}.csv"
    )

def two_rotors_time_series_path(variant: Variant, angular_position: float, solver: Solver) -> Path:
    return two_rotors_time_series_folder(solver) / (
        f"{variant.value}{ANGULAR_POSITION_SEPARATOR}{float(angular_position)}.csv"
    )

def results_plot_path(solver: Solver) -> Path:
    return solver.folder / f"results_{solver.value}.png"

def mesh_convergence_plot_path(spin_ratio: float, solver: Solver) -> Path:
    return solver.folder / (
        f"mesh_convergence_{solver.value}{SPIN_RATIO_SEPARATOR}{float(spin_ratio)}.png"
    )

def interference_plot_path(solver: Solver) -> Path:
    return solver.folder / f"interference_factors_{solver.value}.png"

def write_time_series(force_history: pd.DataFrame, file_path: Path) -> None:
    '''
    Stores the force history of a case. The full force history is stored, and the mean values are
    calculated from it when plotting.
    '''
    file_path.parent.mkdir(exist_ok=True)

    force_history.to_csv(file_path, index=False)

def read_interference_data() -> pd.DataFrame:
    '''
    The interference factors from the experiments by Deybach et al. (2026)
    '''
    return pd.read_csv(INTERFERENCE_DATA_PATH, sep=";", encoding="utf-8-sig")

def experimental_angular_positions() -> list[float]:
    return read_interference_data()["Angular position of the interfering rotor [deg]"].tolist()

def mean_coefficients(time_series: pd.DataFrame) -> dict[str, float]:
    return rotor_setup.force_coefficients(
        time_series["time"].tolist(), time_series["force.x"].tolist(), time_series["force.y"].tolist()
    )

def mean_results_from_time_series(solver: Solver) -> dict:
    '''
    Calculates the mean force coefficients from all stored time series. The results are returned as
    results[variant][spin_ratio] = {"cl": ..., "cd": ...}. Files that do not match a known variant
    are skipped.
    '''
    results: dict = {}

    for file_path in sorted(time_series_folder(solver).glob(f"*{SPIN_RATIO_SEPARATOR}*.csv")):
        variant_value, spin_ratio = file_path.stem.split(SPIN_RATIO_SEPARATOR)

        try:
            variant = Variant(variant_value)
        except ValueError:
            continue

        results.setdefault(variant.value, {})[spin_ratio] = mean_coefficients(
            pd.read_csv(file_path)
        )

    return results

def mesh_convergence_results_from_time_series(spin_ratio: float, solver: Solver) -> dict:
    '''
    Calculates the mean force coefficients from all stored time series from the mesh convergence
    test at the given spin ratio. The results are returned as
    results[variant][cells_per_chord_length] = {"cl": ..., "cd": ...}
    '''
    results: dict = {}

    for variant in Variant:
        prefix = f"{variant.value}{SPIN_RATIO_SEPARATOR}{float(spin_ratio)}{CELLS_SEPARATOR}"

        for file_path in sorted(mesh_convergence_time_series_folder(solver).glob(f"{prefix}*.csv")):
            cells_per_chord_length = file_path.stem.removeprefix(prefix)

            results.setdefault(variant.value, {})[cells_per_chord_length] = mean_coefficients(
                pd.read_csv(file_path)
            )

    return results

def interference_factors_from_time_series(
    variant: Variant,
    solver: Solver
) -> dict[float, dict[str, float]]:
    '''
    Calculates the interference factors from all stored time series from the two rotor case for the
    given variant, using the stored single rotor case at the same spin ratio. The results are
    returned as results[angular_position] = {"cl": ..., "cd": ...}. The results are empty if there
    is no stored single rotor case.
    '''
    single_path = time_series_path(variant, rotor_setup.TWO_ROTORS_SPIN_RATIO, solver)

    if not single_path.exists():
        return {}

    single_coefficients = mean_coefficients(pd.read_csv(single_path))

    prefix = f"{variant.value}{ANGULAR_POSITION_SEPARATOR}"

    results: dict[float, dict[str, float]] = {}

    for file_path in two_rotors_time_series_folder(solver).glob(f"{prefix}*.csv"):
        coefficients = mean_coefficients(pd.read_csv(file_path))

        results[float(file_path.stem.removeprefix(prefix))] = {
            coefficient: coefficients[coefficient] / single_coefficients[coefficient]
            for coefficient in ("cl", "cd")
        }

    return dict(sorted(results.items()))

def lifting_line_interference_factors(
    angular_positions: list[float]
) -> tuple[np.ndarray, np.ndarray]:
    '''
    Computes the drag and lift interference factors with the quasi-steady lifting line model from
    the lifting line example. The example folder is added to the end of the path, so that it does
    not shadow any of the modules in this folder.
    '''
    if str(LIFTING_LINE_EXAMPLE_FOLDER) not in sys.path:
        sys.path.append(str(LIFTING_LINE_EXAMPLE_FOLDER))

    import two_rotors_interference_factors as lifting_line_example

    return lifting_line_example.simulate_interference_factors(
        lifting_line_example.QUASI_STEADY_VARIANT, np.array(angular_positions)
    )

def show_or_save(fig: plt.Figure, file_path: Path | None) -> None:
    '''
    Shows the figure, or saves it to the file path if one is given
    '''
    if file_path is None:
        plt.show()
    else:
        fig.savefig(file_path, dpi=300, bbox_inches='tight')

    plt.close(fig)

def plot_time_series(
    time_series: pd.DataFrame,
    solver_name: str,
    title: str,
    file_path: Path | None = None
) -> None:
    '''
    Plots the lift and drag coefficients as a function of time for a single case, together with the
    mean values over the averaging window.
    '''
    means = mean_coefficients(time_series)

    time = time_series["time"].to_numpy()

    coefficients = {
        "cl": time_series["force.y"].to_numpy() / rotor_setup.force_factor(),
        "cd": time_series["force.x"].to_numpy() / rotor_setup.force_factor(),
    }

    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(16, 6))

    for ax, coefficient, label in ((ax1, "cl", "Lift coefficient"), (ax2, "cd", "Drag coefficient")):
        ax.plot(time, coefficients[coefficient], label=solver_name)

        ax.axvspan(
            time[-1] - rotor_setup.AVERAGING_TIME, time[-1], color="grey", alpha=0.2,
            label="Averaging window"
        )
        ax.axhline(
            means[coefficient], color="black", linestyle="--",
            label=f"Mean = {means[coefficient]:.3f}"
        )

        ax.set_xlabel("Time [s]")
        ax.set_ylabel(label)
        ax.legend()

    ax1.set_title(title)

    show_or_save(fig, file_path)

def plot_results(results: dict, solver_name: str, file_path: Path | None = None) -> None:
    '''
    Plots the lift as a function of the spin ratio, and the drag as a function of the lift, for each
    variant in the results, together with the comparison data. The results are stored as
    results[variant][spin_ratio] = {"cl": ..., "cd": ...}
    '''
    comparison_data = pd.read_csv(COMPARISON_DATA_PATH)

    w_plot = 18
    h_plot = w_plot / 2.35
    fig = plt.figure(figsize=(w_plot, h_plot))

    ax1 = fig.add_subplot(121)
    ax2 = fig.add_subplot(122)

    for variant in Variant:
        if variant.value not in results:
            continue

        variant_results = sorted(
            (float(spin_ratio), values) for spin_ratio, values in results[variant.value].items()
        )

        spin_ratios = [spin_ratio for spin_ratio, _ in variant_results]
        cl = [values["cl"] for _, values in variant_results]
        cd = [values["cd"] for _, values in variant_results]

        label = f"{solver_name}, {variant.label.lower()}"

        ax1.plot(spin_ratios, cl, "-s", label=label)
        ax2.plot(cl, cd, "-s", label=label)

    # Comparison against theoretical values for a lift-induced drag on a lifting surface standing
    # on a symmetry plane with the same aspect ratio as the rotor
    cl_theory = np.linspace(0, 8.0, 100)
    cd0 = 0.43
    ax2.plot(
        cl_theory,
        cd0 + cl_theory**2/(np.pi * 2.0 * ASPECT_RATIO),
        "--",
        color="grey",
        label="Theoretical drag, elliptic wing"
    )

    ax1.plot(
        comparison_data["spin_ratios"],
        comparison_data["cl"],
        "-o",
        label="Deybach et al. (2024)"
    )

    ax2.plot(
        comparison_data["cl"],
        comparison_data["cd"],
        "-o",
        label="Deybach et al. (2024)"
    )

    ax1.set_xlim(0.0, 4.25)
    ax1.set_ylim(0.0, 9.5)

    ax2.set_xlim(0.0, 9.5)
    ax2.set_ylim(0.0, 5.0)

    ax1.set_xlabel("Spin ratio")
    ax1.set_ylabel("Lift coefficient")

    ax2.set_xlabel("Lift coefficient")
    ax2.set_ylabel("Drag coefficient")

    ax2.legend(loc=2)

    show_or_save(fig, file_path)

def plot_mesh_convergence(
    results: dict,
    spin_ratio: float,
    solver_name: str,
    file_path: Path | None = None
) -> None:
    '''
    Plots the lift and drag coefficients as a function of the grid resolution at the rotor, for each
    variant in the results, together with the comparison data at the same spin ratio. The results are
    stored as results[variant][cells_per_chord_length] = {"cl": ..., "cd": ...}
    '''
    comparison_data = pd.read_csv(COMPARISON_DATA_PATH)

    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(16, 6))

    for variant in Variant:
        if variant.value not in results:
            continue

        variant_results = sorted(
            (int(cells), values) for cells, values in results[variant.value].items()
        )

        cells = [cells for cells, _ in variant_results]

        ax1.plot(cells, [values["cl"] for _, values in variant_results], "-s", label=variant.label)
        ax2.plot(cells, [values["cd"] for _, values in variant_results], "-s", label=variant.label)

    for ax, coefficient in ((ax1, "cl"), (ax2, "cd")):
        reference = np.interp(spin_ratio, comparison_data["spin_ratios"], comparison_data[coefficient])

        ax.axhline(reference, color="black", linestyle="--", label="Deybach et al. (2024)")

        ax.set_xscale("log", base=2)
        ax.set_xlabel("Cells per chord length")

    ax1.set_ylabel("Lift coefficient")
    ax2.set_ylabel("Drag coefficient")

    ax1.set_title(f"{solver_name}, spin ratio {spin_ratio}")
    ax1.legend()

    show_or_save(fig, file_path)

def plot_interference_factors(
    variants: list[Variant],
    solver: Solver,
    file_path: Path | None = None
) -> None:
    '''
    Plots the interference factors for the given variants as a function of the angular position of
    the interfering rotor, together with the quasi-steady lifting line results and the experimental
    data from Deybach et al. (2026). The lifting line results are computed for all the angular
    positions in the experiments, while the actuator line results are plotted for the angular
    positions that are stored.
    '''
    default_colors = plt.rcParams['axes.prop_cycle'].by_key()['color']
    markers = ["s", "^", "v", "D"]

    interference_data = read_interference_data()
    angular_positions = experimental_angular_positions()

    cd_lifting_line, cl_lifting_line = lifting_line_interference_factors(angular_positions)

    w_plot = 16
    h_plot = w_plot / 1.85
    fig = plt.figure(figsize=(w_plot, h_plot))

    ax = fig.add_subplot(111)

    for variant_index, variant in enumerate(variants):
        results = interference_factors_from_time_series(variant, solver)

        if len(results) == 0:
            print(f"No stored two rotor results for {variant.label}")

            continue

        for color, coefficient in ((default_colors[0], "cd"), (default_colors[1], "cl")):
            ax.plot(
                list(results.keys()),
                [values[coefficient] for values in results.values()],
                "--" + markers[variant_index % len(markers)],
                color=color,
                label=f"{coefficient.upper()} interference, {solver.label}, {variant.label}"
            )

    ax.plot(
        angular_positions, cd_lifting_line,
        "-",
        color=default_colors[0],
        label="CD interference, lifting line, quasi-steady"
    )

    ax.plot(
        angular_positions, cl_lifting_line,
        "-",
        color=default_colors[1],
        label="CL interference, lifting line, quasi-steady"
    )

    ax.scatter(
        angular_positions,
        interference_data["IFD"],
        color=default_colors[0],
        label="CD interference, experiment, Deybach et al. (2026)"
    )

    ax.scatter(
        angular_positions,
        interference_data["IFL"],
        color=default_colors[1],
        label="CL interference, experiment, Deybach et al. (2026)"
    )

    ax.set_xlabel("Angular position of the interfering rotor [deg]")
    ax.set_ylabel("Interference factor")

    ax.set_ylim(0.5, 2.5)
    ax.set_xlim(-180, 180)

    ax.legend()

    show_or_save(fig, file_path)

def main(default_solver: Solver) -> None:
    '''
    The command line interface for the plots. The OpenFOAM example calls this with OpenFOAM as the
    default solver.
    '''
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--save", action="store_true", help="Save the plot to a file instead of showing it"
    )
    parser.add_argument(
        "--solver", default=default_solver.value, choices=[solver.value for solver in Solver],
        help="The solver to plot the results from"
    )
    subparsers = parser.add_subparsers(dest="command", required=True)

    subparsers.add_parser(
        "means", help="Plot the mean values of all stored simulations against the experimental data"
    )

    time_series_parser = subparsers.add_parser("time-series", help="Plot a single time series")
    time_series_parser.add_argument(
        "--spin-ratio", type=float, default=None,
        help="Spin ratio of the case to plot. Not needed for the two rotor case."
    )
    time_series_parser.add_argument(
        "--variant", default=rotor_setup.DEFAULT_VARIANT.value,
        choices=[variant.value for variant in Variant], help="Variant of the case to plot"
    )
    time_series_parser.add_argument(
        "--cells-per-chord-length", type=int, default=None,
        help="Plot the case from the mesh convergence test with this resolution"
    )
    time_series_parser.add_argument(
        "--angular-position", type=float, default=None,
        help="Plot the two rotor case with the interfering rotor at this angular position"
    )

    mesh_convergence_parser = subparsers.add_parser(
        "mesh-convergence", help="Plot the results of the mesh convergence test"
    )
    mesh_convergence_parser.add_argument(
        "--spin-ratio", type=float, default=rotor_setup.MESH_TEST_SPIN_RATIO,
        help="Spin ratio of the mesh convergence test"
    )
    rotor_setup.add_variant_argument(mesh_convergence_parser, "plot")

    interference_parser = subparsers.add_parser(
        "interference",
        help="Plot the interference factors for two rotors against the lifting line results and "
        "the experimental data"
    )
    rotor_setup.add_variant_argument(interference_parser, "plot")

    args = parser.parse_args()

    solver = Solver(args.solver)

    match args.command:
        case "means":
            plot_results(
                mean_results_from_time_series(solver),
                solver.label,
                results_plot_path(solver) if args.save else None
            )
        case "time-series":
            variant = Variant(args.variant)

            if args.angular_position is not None:
                time_series_file_path = two_rotors_time_series_path(
                    variant, args.angular_position, solver
                )
                title = (
                    f"{solver.label}, {variant.label}, two rotors, angular position "
                    f"{args.angular_position}"
                )
            elif args.spin_ratio is None:
                parser.error("--spin-ratio is required, unless --angular-position is given")
            elif args.cells_per_chord_length is None:
                time_series_file_path = time_series_path(variant, args.spin_ratio, solver)
                title = f"{solver.label}, {variant.label}, spin ratio {args.spin_ratio}"
            else:
                time_series_file_path = mesh_convergence_time_series_path(
                    variant, args.spin_ratio, args.cells_per_chord_length, solver
                )
                title = (
                    f"{solver.label}, {variant.label}, spin ratio {args.spin_ratio}, "
                    f"{args.cells_per_chord_length} cells per chord length"
                )

            plot_time_series(
                pd.read_csv(time_series_file_path),
                solver.label,
                title,
                time_series_file_path.with_suffix(".png") if args.save else None
            )
        case "mesh-convergence":
            variant_values = [
                variant.value for variant in rotor_setup.variants_from_arguments(args.variant)
            ]

            results = {
                variant_value: values
                for variant_value, values in mesh_convergence_results_from_time_series(
                    args.spin_ratio, solver
                ).items()
                if variant_value in variant_values
            }

            plot_mesh_convergence(
                results,
                args.spin_ratio,
                solver.label,
                mesh_convergence_plot_path(args.spin_ratio, solver) if args.save else None
            )
        case "interference":
            plot_interference_factors(
                rotor_setup.variants_from_arguments(args.variant),
                solver,
                interference_plot_path(solver) if args.save else None
            )


if __name__ == '__main__':
    main(Solver.Stormflow)
