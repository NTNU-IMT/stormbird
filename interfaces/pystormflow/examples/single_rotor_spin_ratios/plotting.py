'''
Plotting of the results from run_simulations.py. The force histories are stored as time series, and
the mean values are calculated from them when plotting. Run as a script to either plot a single time
series, or the mean values of all stored simulations against the data from Deybach et al. (2024):

    python plotting.py means
    python plotting.py time-series --spin-ratio 3.0 --variant tip_loss_correction

The plots are shown directly by default. Use --save (before the command) to save them to file instead.
'''

import argparse
from pathlib import Path

import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

import rotor_setup
from rotor_setup import Variant

COMPARISON_DATA_PATH = (
    Path(__file__).resolve().parents[3] / "pystormbird" / "examples" / "3 - rotor_sails" / "data" /
    "deybach_2024_single_rotor.csv"
)

TIME_SERIES_FOLDER = Path(__file__).parent / "time_series_stormflow"
RESULTS_PLOT_PATH = Path(__file__).parent / "results_stormflow.png"

SPIN_RATIO_SEPARATOR = "_spin_ratio_"

def time_series_path(variant: Variant, spin_ratio: float) -> Path:
    return TIME_SERIES_FOLDER / f"{variant.value}{SPIN_RATIO_SEPARATOR}{float(spin_ratio)}.csv"

def read_time_series(variant: Variant, spin_ratio: float) -> pd.DataFrame:
    return pd.read_csv(time_series_path(variant, spin_ratio))

def mean_coefficients(time_series: pd.DataFrame) -> dict[str, float]:
    return rotor_setup.force_coefficients(
        time_series["time"].tolist(), time_series["force.x"].tolist(), time_series["force.y"].tolist()
    )

def mean_results_from_time_series() -> dict:
    '''
    Calculates the mean force coefficients from all stored time series. The results are returned as
    results[variant][spin_ratio] = {"cl": ..., "cd": ...}. Files that do not match a known variant
    are skipped.
    '''
    results: dict = {}

    for file_path in sorted(TIME_SERIES_FOLDER.glob(f"*{SPIN_RATIO_SEPARATOR}*.csv")):
        variant_value, spin_ratio = file_path.stem.split(SPIN_RATIO_SEPARATOR)

        try:
            variant = Variant(variant_value)
        except ValueError:
            continue

        results.setdefault(variant.value, {})[spin_ratio] = mean_coefficients(
            pd.read_csv(file_path)
        )

    return results

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
    variant: Variant,
    spin_ratio: float,
    solver_name: str,
    file_path: Path | None = None
) -> None:
    '''
    Plots the lift and drag coefficients as a function of time for a single case, together with the
    mean values over the averaging window.
    '''
    time_series = read_time_series(variant, spin_ratio)
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

    ax1.set_title(f"{solver_name}, {variant.label}, spin ratio {spin_ratio}")

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

    ax1.legend(loc=4)

    show_or_save(fig, file_path)

def plot_mesh_convergence(results: dict, spin_ratio: float, solver_name: str, file_path: Path) -> None:
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

    fig.savefig(file_path, dpi=300, bbox_inches='tight')

    plt.close(fig)


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--save", action="store_true", help="Save the plot to a file instead of showing it"
    )
    subparsers = parser.add_subparsers(dest="command", required=True)

    subparsers.add_parser(
        "means", help="Plot the mean values of all stored simulations against the experimental data"
    )

    time_series_parser = subparsers.add_parser("time-series", help="Plot a single time series")
    time_series_parser.add_argument(
        "--spin-ratio", type=float, required=True, help="Spin ratio of the case to plot"
    )
    time_series_parser.add_argument(
        "--variant", default=Variant.TipLossCorrection.value,
        choices=[variant.value for variant in Variant], help="Variant of the case to plot"
    )

    args = parser.parse_args()

    match args.command:
        case "means":
            plot_results(
                mean_results_from_time_series(),
                "Stormflow",
                RESULTS_PLOT_PATH if args.save else None
            )
        case "time-series":
            variant = Variant(args.variant)

            plot_time_series(
                variant,
                args.spin_ratio,
                "Stormflow",
                time_series_path(variant, args.spin_ratio).with_suffix(".png") if args.save else None
            )
