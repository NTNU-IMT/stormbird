"""
Copyright (C) 2026, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

Report of the results from simulations of sails on a ship, as an HTML file with interactive
figures, made from a folder with result files, and optionally reference data. Requires plotly,
which is an optional dependency (install with the "report" extra).

The result files must be named with the naming convention in naming.py, and contain the simulation
builder that was used, as stored by the run functions in standard_simulations. Other files are
skipped. The results can be from several setups, solvers and test conditions. Reference data, e.g.,
from experiments, is read from the files in the same folder that end in REFERENCE_FILE_SUFFIX, or
from the given files, and is matched to the cases with the same setup name and test condition (see
reference_data.py).

The cases are grouped in sources, where a source is the setup, the solver, the wind and ship
velocities, the wind profile, and whether the sail settings are fixed or set by the controller. The
values of the fixed sail settings, i.e., the angles of attack and the internal state, are not part
of the source, as they often vary with the wind direction. They are shown in the hover text and in
the detailed view instead. A different controller setup should therefore be a different setup name.
With the option combine_wind_velocities, the true wind velocity is not part of the source either,
so that test conditions with different wind velocities are shown as one source. The wind velocity is
then shown in the hover text.
Each wing has its own color, the same in all figures. The simulations are shown as lines without
markers in the overview and the detailed view, so that they are easy to tell apart from the reference
values, which are shown as markers. In the overview, each source has its own line style, and
marker style for the reference values. A source with only one case is shown as a short horizontal
dash, as a line through a single point is not visible. The cases in a
source are connected with lines, sorted by the apparent wind direction. Several cases with the same
wind direction, e.g., with different angles of attack, are connected in the order of the angles of
attack. The overview and the comparison have buttons to show either the force coefficients of each
wing, which is the default, or of all wings together.

When there are several setups, a single dropdown menu at the top of the page chooses the setup, and
all the figures below only show the cases of that setup.

1) Overview, as a function of the apparent wind direction at mid-span:
   - the mean thrust coefficient (force along the forward direction of the ship) of each wing, and
     of all wings together, with the reference values as markers
   - the mean side force coefficient (force towards port), in the same way
   - the effective angle of attack of each wing, i.e., the angle used in the sectional model,
     including the induced velocities, as the median over the wing and the mean over the full
     results in the averaging window
2) Comparison with the reference data, if any: the simulated against the reference thrust and side
   force coefficients of each wing, and of all wings together, for all cases with reference data.
3) Detailed view of a single case, chosen from a dropdown menu:
   - the spanwise distributions of the circulation strength and the effective angle of attack, from
     the last full result, together with the target angle: the fixed geometric angle of attack, or
     the angle of attack set point of the controller
   - the time history of the thrust and the side force coefficients, with a vertical line at the
     start of the averaging window, as a check of the convergence
   - the mean force coefficients of the simulation and the reference data, as numbers in the title

All angles of attack and circulation strengths are shown with the sign that gives thrust, i.e.,
positive values give lift with a positive thrust component, as in the test conditions. Note that the
effective angle of attack can be negative for sails with a large lift at zero angle of attack, such as
suction sails, as the induced velocities then are large.

The force coefficients of each wing are made non-dimensional with the dynamic pressure of the
undisturbed apparent wind at the mid-span of the wing, and the projected area of the wing. The total
coefficients are the sum of the forces on all wings, divided by the sum of the dynamic pressure
times the projected area of each wing, so that they are consistent with the coefficients of each
wing. The forces in the results are forces per unit density, so the density in the dynamic pressure
is ACTUATOR_LINE_FORCE_DENSITY. The means are taken over the last part of the simulated time, given
by the averaging fraction. For the steady OpenFOAM simulations, the time is the iteration number.
"""

import math
import warnings
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

import numpy as np

from ...spatial_vector import SpatialVector
from ...wind import WindCondition, WindEnvironment
from ...actuator_line import ActuatorLineBuilder
from ...line_force_model import WingBuilder
from ...openfoam.results import ActuatorLineResults
from ..actuator_line_runs import ACTUATOR_LINE_FORCE_DENSITY
from ..plot_utils import (
    add_data_toggle, add_dropdown, averaging_start_index, lock_subplot_aspect_ratio, write_html,
    write_html_with_selector
)

from .conditions import ShipTestCondition, WindProfile
from .naming import parse_case_name, condition_key
from .reference_data import ReferenceData, REFERENCE_FILE_SUFFIX, reference_files_in_folder
from .simulation_settings import CfdSolver
from .sails_on_ship import thrust_sign, _mid_span_point, _non_virtual_end_points

DEFAULT_AVERAGING_FRACTION = 0.25
"""Fraction of the simulated time, counted from the end, that the mean values are computed over"""

DEFAULT_REPORT_FILE_NAME = "sails_on_ship_report.html"

REPORT_TITLE = "Stormbird sails-on-ship standard report"

# Line and marker styles for each source in the overview. The wings have different colors.
_SOURCE_DASH = ["solid", "dash", "dot", "dashdot", "longdash", "longdashdot"]
_SOURCE_MARKER = ["circle", "diamond", "square", "triangle-up", "cross", "x"]
_TOTAL_MARKER = "star"
_TOTAL_COLOR = "#636EFA"
"""The color of the total of all wings in the overview, which is the same as for the first wing, as
they are not shown at the same time"""

# Marker fill for each reference data set in the comparison
_REFERENCE_FILLS = ["", "-open", "-dot", "-open-dot"]
_REFERENCE_FILL_NAMES = ["filled", "open", "dot", "open with dot"]

# Options of the buttons that choose between the coefficients of each wing and of all wings together
_WING_TOGGLE_LABELS = ["Wings", "Total"]

SUBPLOT_WIDTH_TO_HEIGHT = 1.85
"""
The aspect ratio of each subplot, as the width divided by the height. The height of the figures is
adjusted to the width of the window, so that this ratio is kept.
"""

# Layout of the top of the figures, in pixels. From the top: an optional dropdown menu, the title,
# and the subplots with their titles. The values are chosen so that these do not overlap.
_DROPDOWN_TOP = 10
_DROPDOWN_HEIGHT = 50
_MENU_HEIGHT = 30
_TITLE_TOP_WITHOUT_DROPDOWN = 25
_TITLE_LINE_HEIGHT = 21
_SUBPLOT_TITLE_SPACE = 50
_BOTTOM_MARGIN = 80

REPORT_MAX_WIDTH = 1600
"""
The maximum width of the figures in the report, in pixels. The figures are centered in wider windows.
"""

# Width of the plot area, in pixels, that the initial height of the figures is computed for, i.e.,
# about the max width without the margins and the legend. The height is adjusted to the real width
# when the report is opened.
_INITIAL_PLOT_WIDTH = REPORT_MAX_WIDTH - 400

# Spacing between the subplots, as fractions of the plot area
_OVERVIEW_SPACING = 0.08
_COMPARISON_SPACING = 0.1
_DETAILED_SPACING = (0.15, 0.09)


@dataclass
class ReportWing:
    '''A wing in a case, with the values needed in the report'''
    label: str
    short_label: str
    area: float
    thrust_sign: float
    """Sign that the angles of attack and circulation strengths are multiplied with"""
    apparent_wind_velocity: float
    """Undisturbed apparent wind velocity at mid-span"""
    target_aoa_deg: float | None
    """The fixed geometric angle of attack or the controller set point, with thrust positive"""
    real_point_indices: list[int]
    """Indices of the control points on the wing itself, i.e., not on virtual segments"""

    @property
    def force_scale(self) -> float:
        '''The dynamic pressure, per unit density, times the projected area'''
        return 0.5 * ACTUATOR_LINE_FORCE_DENSITY * self.apparent_wind_velocity**2 * self.area


@dataclass
class ReportReference:
    '''The reference values for a case, from one reference data set'''
    label: str
    coefficients: dict[str, list[float | None]]
    """The "thrust" and "side_force" coefficients of each wing, None if not available"""
    total: dict[str, float | None]
    """The total "thrust" and "side_force" coefficients, None if not available for all wings"""


@dataclass
class ReportCase:
    '''The processed results of a case'''
    file_path: Path
    setup_name: str | None
    solver: CfdSolver
    condition: ShipTestCondition
    wings: list[ReportWing]
    target_label: str
    apparent_wind_direction_deg: float
    time: np.ndarray
    history: dict[str, list[np.ndarray]]
    """Time histories of the "thrust" and "side_force" coefficients of each wing"""
    mean: dict[str, list[float]]
    """Mean "thrust" and "side_force" coefficients of each wing"""
    total_mean: dict[str, float]
    """Mean "thrust" and "side_force" coefficients of all wings together"""
    effective_aoa_deg: list[float]
    averaging_start_time: float
    distributions: list[dict[str, np.ndarray]] | None
    """Spanwise "height", "circulation" and "aoa_deg" of each wing, from the last full result"""
    distribution_time: float
    references: list[ReportReference] = field(default_factory=list)
    combine_wind_velocities: bool = False
    """
    If True, cases that only differ in the true wind velocity belong to the same source, see
    source_key
    """

    @property
    def true_wind_direction_deg(self) -> float:
        return round(math.degrees(self.condition.true_wind_direction), 1)

    def key(self) -> str:
        '''The setup name and the rounded test condition, used to match reference data'''
        return condition_key(self.condition, self.setup_name)

    def source_key(self) -> tuple[Any, ...]:
        '''
        The case name without the true wind direction and the values of the fixed sail settings,
        see the module documentation. The true wind velocity is also left out, i.e., None, if
        combine_wind_velocities is True.
        '''
        condition = self.condition

        return (
            self.setup_name,
            self.solver,
            None if self.combine_wind_velocities else condition.true_wind_velocity,
            condition.ship_velocity,
            condition.wind_profile,
            condition.angle_of_attack is not None,
        )

    def sail_settings_text(self) -> str:
        '''The fixed sail settings, e.g., "AoA 10.0/12.5 deg, IS 0.1", or "" if not set'''
        parts = []

        aoa_deg = _rounded_angles_deg(self.condition)

        if aoa_deg is not None:
            parts.append("AoA " + "/".join(f"{value:.1f}" for value in aoa_deg) + " deg")

        if self.condition.internal_state is not None:
            parts.append(f"IS {self.condition.internal_state:g}")

        return ", ".join(parts)


def _rounded_angles_deg(condition: ShipTestCondition) -> tuple[float, ...] | None:
    '''The fixed angles of attack in degrees, rounded as in the case names, or None if not set'''
    if condition.angle_of_attack is None:
        return None

    angles = (
        condition.angle_of_attack if isinstance(condition.angle_of_attack, list)
        else [condition.angle_of_attack]
    )

    return tuple(round(math.degrees(angle), 1) for angle in angles)


def _source_sort_key(source: tuple[Any, ...]) -> tuple[Any, ...]:
    '''Sorts by setup and solver, with the controller before the fixed sail settings'''
    setup_name, solver, true_wind_velocity, ship_velocity, wind_profile, fixed = source

    return (
        setup_name or "",
        list(CfdSolver).index(solver),
        # None when the wind velocities are combined, see ReportCase.source_key
        -1.0 if true_wind_velocity is None else true_wind_velocity,
        ship_velocity,
        list(WindProfile).index(wind_profile),
        fixed,
    )


def _source_labels(sources: list[tuple[Any, ...]]) -> dict[tuple[Any, ...], str]:
    '''
    Labels for the sources, with the solver and the type of sail settings, and the other parts only
    if they differ between the sources
    '''
    varying = [len(set(values)) > 1 for values in zip(*sources)] if sources else []

    labels = {}

    for source in sources:
        setup_name, solver, true_wind_velocity, ship_velocity, wind_profile, fixed = source

        parts = []

        if varying[0] and setup_name is not None:
            parts.append(setup_name)

        parts.append(solver.value)

        if varying[2]:
            parts.append(f"TWV {true_wind_velocity:.2f} m/s")

        if varying[3]:
            parts.append(f"SV {ship_velocity:.2f} m/s")

        if varying[4]:
            parts.append("constant wind" if wind_profile == WindProfile.Constant else "power law wind")

        parts.append("fixed sail settings" if fixed else "controller")

        labels[source] = ", ".join(parts)

    return labels


def _simulation_line_mode(nr_points: int, color: str) -> dict[str, Any]:
    '''
    The mode and marker of a simulation trace in the overview. The simulations are shown as lines,
    so that they are easy to tell apart from the reference values, which are markers. A line through
    a single point is not visible, so a source with only one case is shown as a short horizontal
    dash instead, which is not one of the marker symbols of the reference values.
    '''
    if nr_points > 1:
        return {"mode": "lines"}

    return {
        "mode": "markers",
        "marker": dict(symbol="line-ew-open", size=16, line=dict(color=color, width=3)),
    }


def _wrap_angle(angle: float) -> float:
    return math.atan2(math.sin(angle), math.cos(angle))


def _real_point_indices(full_result: dict[str, Any], wing_index: int, wing_builder: WingBuilder) -> list[int]:
    '''The control points of a wing that are on the wing itself, and not on virtual segments'''
    wing_range = full_result["wing_indices"][wing_index]

    start, end = _non_virtual_end_points(wing_builder)
    span = end - start
    span_length_squared = span.length_squared()

    indices = []

    for index in range(wing_range["start"], wing_range["end"]):
        point = SpatialVector.from_dict(full_result["ctrl_points"][index])

        fraction = (point - start).dot(span) / span_length_squared

        if -1e-6 <= fraction <= 1.0 + 1e-6:
            indices.append(index)

    return indices


def _target_aoa_deg(
    condition: ShipTestCondition,
    actuator_line: ActuatorLineBuilder,
    wing_index: int,
    sign: float
) -> float | None:
    '''
    The fixed geometric angle of attack, or the angle of attack set point of the controller, with
    thrust positive. The set point is only given for controllers with a single set point, as it
    otherwise varies with the apparent wind direction.
    '''
    angles_of_attack = condition.angles_of_attack(len(actuator_line.line_force_model.wing_builders))

    if angles_of_attack is not None:
        return math.degrees(angles_of_attack[wing_index])

    if actuator_line.controller is None:
        return None

    set_points = actuator_line.controller.set_points[wing_index]

    if set_points.angle_of_attack_data is None or len(set_points.angle_of_attack_data) != 1:
        return None

    return sign * math.degrees(set_points.angle_of_attack_data[0])


def _target_label(condition: ShipTestCondition, actuator_line: ActuatorLineBuilder) -> str:
    if condition.angle_of_attack is not None:
        return "fixed geometric angle of attack"

    if actuator_line.controller is None:
        return "no target"

    set_points = actuator_line.controller.set_points

    if len(set_points) > 0 and set_points[0].use_effective_angle_of_attack:
        return "controller set point (effective angle of attack)"

    return "controller set point (geometric angle of attack)"


def read_report_case(
    file_path: Path | str,
    averaging_fraction: float = DEFAULT_AVERAGING_FRACTION
) -> ReportCase | None:
    '''
    Reads and processes a result file. Returns None, with a warning, if the name does not follow the
    naming convention, or if the file does not contain the simulation builder.
    '''
    file_path = Path(file_path)

    parsed = parse_case_name(file_path.name)

    if parsed is None:
        warnings.warn(f"Skipping {file_path.name}, as the name does not follow the naming convention")

        return None

    results = ActuatorLineResults.from_json_file(file_path)

    builder = results.simulation_builder

    if builder is None:
        warnings.warn(f"Skipping {file_path.name}, as it does not contain the simulation builder")

        return None

    wind_environment = WindEnvironment.model_validate(builder.get("wind_environment", {}))
    wind_condition = WindCondition.model_validate(builder["wind_condition"])
    linear_velocity = SpatialVector.model_validate(builder["linear_velocity"])
    actuator_line = ActuatorLineBuilder.model_validate(builder["actuator_line"])

    condition = parsed.condition

    forward = (wind_environment.zero_direction_vector * -1.0).normalize()
    up = wind_environment.up_direction.normalize()
    port = up.cross(forward)

    wing_builders = actuator_line.line_force_model.wing_builders

    mid_span_points = [_mid_span_point(wing_builder) for wing_builder in wing_builders]

    apparent_winds = [
        wind_environment.steady_apparent_wind_velocity_vector_at_location(
            wind_condition, point, linear_velocity
        )
        for point in mid_span_points
    ]

    mean_mid_span_height = float(np.mean([
        wind_environment.height_from_location(point) for point in mid_span_points
    ]))

    apparent_wind_direction = wind_environment.apparent_wind_direction_from_condition_and_linear_velocity(
        wind_condition, linear_velocity, height=mean_mid_span_height
    )

    full_result_list = [results.full_results[key] for key in sorted(results.full_results)]

    wings = []

    for wing_index, (wing_builder, point, apparent_wind) in enumerate(
        zip(wing_builders, mid_span_points, apparent_winds)
    ):
        sign = thrust_sign(forward, wing_builder, apparent_wind)

        side_component = point.dot(port)

        if abs(side_component) < 1e-6:
            side = "centerline"
        else:
            side = "port" if side_component > 0.0 else "starboard"

        wings.append(ReportWing(
            label = f"Wing {wing_index} ({side})",
            short_label = f"W{wing_index}",
            area = wing_builder.projected_area(),
            thrust_sign = sign,
            apparent_wind_velocity = apparent_wind.length(),
            target_aoa_deg = _target_aoa_deg(condition, actuator_line, wing_index, sign),
            real_point_indices = (
                _real_point_indices(full_result_list[0], wing_index, wing_builder)
                if full_result_list else []
            ),
        ))

    # Force coefficient histories
    time = np.array(results.forces["time"])
    start = averaging_start_index(time, averaging_fraction)
    averaging_start_time = float(time[start])

    history: dict[str, list[np.ndarray]] = {"thrust": [], "side_force": []}

    for wing_index, wing in enumerate(wings):
        force = [
            np.array(results.forces[f"force_{wing_index}.{component}"]) for component in "xyz"
        ]

        dynamic_pressure = 0.5 * ACTUATOR_LINE_FORCE_DENSITY * wing.apparent_wind_velocity**2

        for key, direction in (("thrust", forward), ("side_force", port)):
            projected = force[0] * direction.x + force[1] * direction.y + force[2] * direction.z

            history[key].append(projected / (dynamic_pressure * wing.area))

    mean = {
        key: [float(np.mean(values[start:])) for values in history[key]] for key in history
    }

    # Total coefficients: the sum of the forces divided by the sum of the force scales
    total_scale = sum(wing.force_scale for wing in wings)

    total_mean = {
        key: float(np.mean(
            np.sum([values * wing.force_scale for values, wing in zip(history[key], wings)], axis=0)[start:]
        )) / total_scale
        for key in history
    }

    # Effective angle of attack, from the full results in the averaging window
    averaged_full_results = [
        full_result for full_result in full_result_list
        if full_result["time"] >= averaging_start_time
    ]

    if len(averaged_full_results) == 0 and len(full_result_list) > 0:
        warnings.warn(
            f"No full results in the last {averaging_fraction:.0%} of the simulated time in "
            f"{file_path.name}. Using the last full result for the angle of attack."
        )

        averaged_full_results = full_result_list[-1:]

    effective_aoa_deg = []

    for wing in wings:
        medians = [
            float(np.median([
                wing.thrust_sign * _wrap_angle(full_result["force_input"]["angles_of_attack"][index])
                for index in wing.real_point_indices
            ]))
            for full_result in averaged_full_results
            if wing.real_point_indices
        ]

        effective_aoa_deg.append(math.degrees(float(np.mean(medians))) if medians else math.nan)

    # Spanwise distributions from the last full result
    distributions: list[dict[str, np.ndarray]] | None = None
    distribution_time = math.nan

    if full_result_list:
        last = full_result_list[-1]
        force_input = last["force_input"]

        distributions = []

        for wing_index, wing in enumerate(wings):
            wing_range = last["wing_indices"][wing_index]
            indices = range(wing_range["start"], wing_range["end"])

            distributions.append({
                "height": np.array([
                    wind_environment.height_from_location(
                        SpatialVector.from_dict(last["ctrl_points"][index])
                    )
                    for index in indices
                ]),
                # Positive circulation gives lift along the velocity crossed with the span, which
                # is the opposite of a positive lift coefficient
                "circulation": -wing.thrust_sign * np.array(
                    [force_input["circulation_strength"][index] for index in indices]
                ),
                "aoa_deg": wing.thrust_sign * np.degrees(
                    [force_input["angles_of_attack"][index] for index in indices]
                ),
            })

        distribution_time = float(last["time"])

    return ReportCase(
        file_path = file_path,
        setup_name = parsed.setup_name,
        solver = parsed.solver,
        condition = condition,
        wings = wings,
        target_label = _target_label(condition, actuator_line),
        apparent_wind_direction_deg = math.degrees(apparent_wind_direction),
        time = time,
        history = history,
        mean = mean,
        total_mean = total_mean,
        effective_aoa_deg = effective_aoa_deg,
        averaging_start_time = averaging_start_time,
        distributions = distributions,
        distribution_time = distribution_time,
    )


def _reference_for_case(case: ReportCase, label: str, point: Any) -> ReportReference | None:
    '''
    The reference values of a reference point for a case, with the total coefficients computed in
    the same way as for the simulations. Returns None, with a warning, if the number of sails does not
    match.
    '''
    if point.nr_sails() != len(case.wings):
        warnings.warn(
            f"Skipping the reference point '{point.key()}' in '{label}', as it has "
            f"{point.nr_sails()} sails, while the simulation has {len(case.wings)}"
        )

        return None

    coefficients: dict[str, list[float | None]] = {
        "thrust": list(point.thrust_coefficients or [None] * len(case.wings)),
        "side_force": list(point.side_force_coefficients or [None] * len(case.wings)),
    }

    total: dict[str, float | None] = {}

    for key, values in coefficients.items():
        if any(value is None for value in values):
            total[key] = None
        else:
            total[key] = (
                sum(float(value) * wing.force_scale for value, wing in zip(values, case.wings) if value is not None)
                / sum(wing.force_scale for wing in case.wings)
            )

    return ReportReference(label=label, coefficients=coefficients, total=total)


def read_report_cases(
    results_folder: Path | str,
    averaging_fraction: float = DEFAULT_AVERAGING_FRACTION,
    reference_files: list[Path | str] | None = None,
    combine_wind_velocities: bool = False,
) -> list[ReportCase]:
    '''
    All cases in the results folder that can be used in the report, sorted by source and then by
    apparent wind direction, with the matching reference values. The reference data is read from
    the given files, or, if not given, from the reference files in the results folder.

    By default, cases with different true wind velocities belong to different sources, i.e., they
    are shown as separate lines in the overview. With combine_wind_velocities, they belong to the
    same source, e.g., when each test condition has its own wind velocity.
    '''
    result_files = [
        file_path for file_path in sorted(Path(results_folder).glob("*.json"))
        if not file_path.name.endswith(REFERENCE_FILE_SUFFIX)
    ]

    cases = [
        case
        for file_path in result_files
        if (case := read_report_case(file_path, averaging_fraction)) is not None
    ]

    for case in cases:
        case.combine_wind_velocities = combine_wind_velocities

    if reference_files is None:
        reference_files = list(reference_files_in_folder(results_folder))

    cases_by_key: dict[str, list[ReportCase]] = {}

    for case in cases:
        cases_by_key.setdefault(case.key(), []).append(case)

    for reference_file in reference_files:
        reference_data = ReferenceData.from_json_file(reference_file)

        nr_unmatched = 0

        for key, point in reference_data.points_by_key().items():
            matching_cases = cases_by_key.get(key, [])

            if not matching_cases:
                nr_unmatched += 1

            for case in matching_cases:
                reference = _reference_for_case(case, reference_data.label, point)

                if reference is not None:
                    case.references.append(reference)

        if nr_unmatched > 0:
            warnings.warn(
                f"{nr_unmatched} of {len(reference_data.points)} points in the reference data "
                f"'{reference_data.label}' have no simulation results"
            )

    cases.sort(
        key=lambda case: (
            _source_sort_key(case.source_key()),
            case.apparent_wind_direction_deg,
            _rounded_angles_deg(case.condition) or (),
            case.condition.internal_state or 0.0,
        )
    )

    return cases


def _source_colors(cases: list[ReportCase]) -> dict[tuple[Any, ...], str]:
    import plotly.colors # type: ignore[import-not-found, import-untyped, unused-ignore]

    palette = plotly.colors.qualitative.Plotly

    sources = sorted(set(case.source_key() for case in cases), key=_source_sort_key)

    return {source: palette[index % len(palette)] for index, source in enumerate(sources)}


def _wing_color(wing_index: int) -> str:
    '''The color of a wing in the comparison and the detailed view'''
    import plotly.colors # type: ignore[import-not-found, import-untyped, unused-ignore]

    palette = plotly.colors.qualitative.Plotly

    return palette[wing_index % len(palette)]


def _case_label(case: ReportCase, source_labels: dict[tuple[Any, ...], str]) -> str:
    parts = [
        source_labels[case.source_key()],
        f"TWD {case.true_wind_direction_deg:.0f} deg (AWD {case.apparent_wind_direction_deg:.1f} deg)",
    ]

    # The wind velocity is not part of the source label when the wind velocities are combined
    if case.combine_wind_velocities:
        parts.append(f"TWV {case.condition.true_wind_velocity:.2f} m/s")

    sail_settings = case.sail_settings_text()

    if sail_settings:
        parts.append(sail_settings)

    return ", ".join(parts)


def _setup_names(cases: list[ReportCase]) -> list[str | None]:
    '''The setup names in the cases, sorted, with None last'''
    names = set(case.setup_name for case in cases)

    return sorted(name for name in names if name is not None) + ([None] if None in names else [])


def _setup_label(setup_name: str | None) -> str:
    return "No setup name" if setup_name is None else setup_name


def _setup_names_text(cases: list[ReportCase]) -> str:
    '''The setup names for the title, or a general text if the cases have no setup name'''
    names = [name for name in _setup_names(cases) if name is not None]

    return ", ".join(names) if names else "sails on ship"


def _values_text(labels: list[str], values: list[float | None], total: float | None) -> str:
    '''The values of each wing and the total, e.g., "W0 0.374, W1 0.729, total 0.552"'''
    parts = [
        f"{label} {'-' if value is None else f'{value:.3f}'}" for label, value in zip(labels, values)
    ]

    parts.append(f"total {'-' if total is None else f'{total:.3f}'}")

    return ", ".join(parts)


def _figure_layout(
    figure: Any,
    rows: int,
    cols: int,
    vertical_spacing: float,
    horizontal_spacing: float,
    nr_title_lines: int,
    has_dropdown: bool
) -> dict[str, Any]:
    '''
    Sets the height of the figure, so that the subplots get the aspect ratio SUBPLOT_WIDTH_TO_HEIGHT,
    and places the title, with room for optional dropdown menus above it, so that the menus, the
    title and the subplot titles do not overlap. The positions are given in pixels from the top, so
    that they do not change when the height is adjusted to the width of the window. Returns the
    position of the menus, as keyword arguments to the functions that add them.
    '''
    title_top = _DROPDOWN_TOP + _DROPDOWN_HEIGHT if has_dropdown else _TITLE_TOP_WITHOUT_DROPDOWN
    top_margin = title_top + nr_title_lines * _TITLE_LINE_HEIGHT + _SUBPLOT_TITLE_SPACE

    subplot_width = _INITIAL_PLOT_WIDTH * (1.0 - horizontal_spacing * (cols - 1)) / cols
    plot_height = (
        rows * subplot_width / SUBPLOT_WIDTH_TO_HEIGHT / (1.0 - vertical_spacing * (rows - 1))
    )

    figure.update_layout(
        height=round(top_margin + plot_height + _BOTTOM_MARGIN),
        template="plotly_white",
        margin=dict(t=top_margin, b=_BOTTOM_MARGIN),
        title=dict(
            x=0.0,
            xref="paper",
            xanchor="left",
            y=1.0,
            yref="container",
            yanchor="top",
            pad=dict(t=title_top),
            font=dict(size=14),
        ),
    )

    lock_subplot_aspect_ratio(figure, SUBPLOT_WIDTH_TO_HEIGHT)

    # The menus are placed above the plot area, with the given distance in pixels
    return dict(y=1.0, yanchor="bottom", pad=dict(b=top_margin - _DROPDOWN_TOP - _MENU_HEIGHT))


def make_overview_figure(
    cases: list[ReportCase],
    averaging_fraction: float = DEFAULT_AVERAGING_FRACTION
) -> Any:
    '''
    Mean values as a function of the apparent wind direction, for all cases, with the reference
    values as markers.
    '''
    import plotly.graph_objects as go # type: ignore[import-not-found, import-untyped, unused-ignore]
    from plotly.subplots import make_subplots # type: ignore[import-not-found, import-untyped, unused-ignore]

    figure = make_subplots(
        rows=3,
        cols=1,
        shared_xaxes=True,
        vertical_spacing=_OVERVIEW_SPACING,
        subplot_titles=(
            "Mean thrust coefficient (+ towards the bow)",
            "Mean side force coefficient (+ towards port)",
            "Effective angle of attack (median over the wing, + gives thrust)",
        )
    )

    sources = sorted(set(case.source_key() for case in cases), key=_source_sort_key)
    source_labels = _source_labels(sources)

    # The thrust and side force traces of the individual wings and of the total, of which only one
    # kind is shown at a time
    wing_traces: list[int] = []
    total_traces: list[int] = []

    def add_trace(trace: Any, row: int, wing_level: bool = False, total: bool = False) -> None:
        if wing_level and row in (1, 2):
            wing_traces.append(len(figure.data))

        if total:
            total_traces.append(len(figure.data))

        figure.add_trace(trace, row=row, col=1)

    reference_labels = sorted(set(
        reference.label for case in cases for reference in case.references
    ))

    # The reference data sets that have a legend entry, for the wings and for the total, as only one
    # of them is shown at a time
    reference_legend_shown: set[tuple[bool, str]] = set()

    def reference_trace(
        points: list[tuple[float, Any]],
        reference_label: str,
        total: bool,
        hover_name: str,
        marker: dict[str, Any],
    ) -> Any:
        '''
        Reference values as markers. All markers from the same reference data are in the same
        legend group, with one legend entry, so they can be shown or hidden together.
        '''
        show_legend = (total, reference_label) not in reference_legend_shown

        reference_legend_shown.add((total, reference_label))

        return go.Scatter(
            x=[point[0] for point in points],
            y=[point[1] for point in points],
            mode="markers",
            name=reference_label,
            legendgroup=f"reference: {reference_label}",
            showlegend=show_legend,
            # After the simulations in the legend, which have the default rank of 1000
            legendrank=2000,
            marker=marker,
            hovertemplate=f"{hover_name}<br>%{{x:.1f}} deg, %{{y:.3f}}<extra></extra>",
        )

    for source_index, source in enumerate(sources):
        source_cases = [case for case in cases if case.source_key() == source]
        source_label = source_labels[source]

        # The source is only part of the legend entries when there are several sources
        source_suffix = f", {source_label}" if len(sources) > 1 else ""

        dash = _SOURCE_DASH[source_index % len(_SOURCE_DASH)]
        marker_symbol = _SOURCE_MARKER[source_index % len(_SOURCE_MARKER)]

        x = [case.apparent_wind_direction_deg for case in source_cases]

        def hover_text(hover_cases: list[ReportCase]) -> list[str]:
            return [_case_label(case, source_labels) for case in hover_cases]

        nr_wings = max(len(case.wings) for case in source_cases)

        for wing_index in range(nr_wings):
            wing_cases = [case for case in source_cases if wing_index < len(case.wings)]

            wing_label = wing_cases[0].wings[wing_index].label
            name = wing_label + source_suffix
            color = _wing_color(wing_index)

            rows = (
                (1, [case.mean["thrust"][wing_index] for case in wing_cases]),
                (2, [case.mean["side_force"][wing_index] for case in wing_cases]),
                (3, [case.effective_aoa_deg[wing_index] for case in wing_cases]),
            )

            for row, y in rows:
                add_trace(
                    go.Scatter(
                        x=[case.apparent_wind_direction_deg for case in wing_cases],
                        y=y,
                        name=name,
                        # One legend entry per wing and source, which toggles it in all rows
                        legendgroup=name,
                        showlegend=(row == 1),
                        line=dict(color=color, dash=dash),
                        text=hover_text(wing_cases),
                        **_simulation_line_mode(len(wing_cases), color),
                    ),
                    row, wing_level=True
                )

            # Reference values of each wing, as markers
            for row, key in ((1, "thrust"), (2, "side_force")):
                for reference_label in reference_labels:
                    points = [
                        (case.apparent_wind_direction_deg, reference.coefficients[key][wing_index])
                        for case in wing_cases
                        for reference in case.references
                        if reference.label == reference_label
                        and reference.coefficients[key][wing_index] is not None
                    ]

                    if points:
                        add_trace(
                            reference_trace(
                                points, reference_label, False,
                                f"{reference_label}, {source_label}, {wing_label}",
                                dict(symbol=marker_symbol, size=9, color=color),
                            ),
                            row, wing_level=True
                        )

        # Totals of all wings, with the reference totals
        total_name = "Total" + source_suffix

        for row, key in ((1, "thrust"), (2, "side_force")):
            add_trace(
                go.Scatter(
                    x=x,
                    y=[case.total_mean[key] for case in source_cases],
                    name=total_name,
                    legendgroup=total_name,
                    showlegend=(row == 1),
                    line=dict(color=_TOTAL_COLOR, dash=dash, width=3),
                    text=hover_text(source_cases),
                    **_simulation_line_mode(len(source_cases), _TOTAL_COLOR),
                ),
                row, total=True
            )

            for reference_label in reference_labels:
                points = [
                    (case.apparent_wind_direction_deg, reference.total[key])
                    for case in source_cases
                    for reference in case.references
                    if reference.label == reference_label and reference.total[key] is not None
                ]

                if points:
                    add_trace(
                        reference_trace(
                            points, reference_label, True,
                            f"{reference_label}, {source_label}, total",
                            dict(symbol=marker_symbol, size=9, color=_TOTAL_COLOR),
                        ),
                        row, total=True
                    )

    figure.update_yaxes(title_text="C_T [-]", row=1, col=1)
    figure.update_yaxes(title_text="C_S [-]", row=2, col=1)
    figure.update_yaxes(title_text="Angle of attack [deg]", row=3, col=1)
    # The x-axis is shown on all the subplots, even though it is shared, so that the values can be
    # read without scrolling down to the bottom subplot
    for row in (1, 2, 3):
        figure.update_xaxes(
            title_text="Apparent wind direction at mid-span [deg] (0 = head wind, + from starboard)",
            showticklabels=True,
            row=row, col=1
        )

    if len(sources) > 1:
        source_text = "Line style: " + ", ".join(
            f"{source_labels[source]} {_SOURCE_DASH[index % len(_SOURCE_DASH)]}"
            for index, source in enumerate(sources)
        )
    else:
        source_text = "Source: " + source_labels[sources[0]] if sources else ""

    # Room for the buttons for the wings and the total
    menu_position = _figure_layout(figure, 3, 1, _OVERVIEW_SPACING, 0.0, 3, has_dropdown=True)

    figure.update_layout(
        title_text=(
            f"<b>Overview</b>: {_setup_names_text(cases)}<br>"
            f"<sub>Means over the last {averaging_fraction:.0%} of the simulated time. Coefficients "
            "relative to the apparent wind at mid-span and the projected area.<br>"
            f"Color: wing. {source_text}. Lines: simulations. Markers: reference values</sub>"
        ),
        hovermode="closest",
        legend=dict(groupclick="togglegroup"),
    )

    # The legend shows the entries of the wings or of the total, depending on what is shown
    add_data_toggle(
        figure, _WING_TOGGLE_LABELS, [total_traces, wing_traces], x=1.0,
        keep_legend=False, **menu_position
    )

    return figure


def make_comparison_figure(cases: list[ReportCase]) -> Any | None:
    '''
    The simulated against the reference thrust and side force coefficients, of each wing and of all
    wings together, for all cases with reference values. The color shows the wing, and the marker
    the solver. None if no case has reference values.
    '''
    import plotly.graph_objects as go # type: ignore[import-not-found, import-untyped, unused-ignore]
    from plotly.subplots import make_subplots # type: ignore[import-not-found, import-untyped, unused-ignore]

    cases_with_references = [case for case in cases if case.references]

    if not cases_with_references:
        return None

    figure = make_subplots(
        rows=1,
        cols=2,
        horizontal_spacing=_COMPARISON_SPACING,
        subplot_titles=(
            "Thrust coefficient",
            "Side force coefficient",
        )
    )

    solver_markers = {CfdSolver.OpenFOAM: "circle", CfdSolver.Stormflow: "square"}

    colors = _source_colors(cases)
    source_labels = _source_labels(list(colors))

    setup_names = _setup_names(cases_with_references)
    reference_labels = sorted(set(
        reference.label for case in cases_with_references for reference in case.references
    ))

    all_values: dict[str, list[float]] = {"thrust": [], "side_force": []}

    # The traces of the individual wings and of the total, of which only one kind is shown at a time
    wing_traces: list[int] = []
    total_traces: list[int] = []

    for setup_name in setup_names:
        setup_cases = [case for case in cases_with_references if case.setup_name == setup_name]

        nr_wings = max(len(case.wings) for case in setup_cases)

        for solver in CfdSolver:
            solver_cases = [case for case in setup_cases if case.solver == solver]

            for reference_index, reference_label in enumerate(reference_labels):
                # Each reference data set has its own marker fill
                reference_fill = _REFERENCE_FILLS[reference_index % len(_REFERENCE_FILLS)]

                # One trace for each wing, and one for the total, in each subplot
                for wing_index in list(range(nr_wings)) + [None]:
                    for col, key in ((1, "thrust"), (2, "side_force")):
                        x, y, text = [], [], []

                        for case in solver_cases:
                            for reference in case.references:
                                if reference.label != reference_label:
                                    continue

                                if wing_index is None:
                                    reference_value = reference.total[key]
                                    value = case.total_mean[key]
                                elif wing_index < len(case.wings):
                                    reference_value = reference.coefficients[key][wing_index]
                                    value = case.mean[key][wing_index]
                                else:
                                    continue

                                if reference_value is None:
                                    continue

                                x.append(reference_value)
                                y.append(value)
                                text.append(_case_label(case, source_labels))

                        if not x:
                            continue

                        all_values[key] += x + y

                        if wing_index is None:
                            wing_name = "Total"
                            color = "black"
                            symbol = _TOTAL_MARKER + reference_fill
                        else:
                            wing_name = next(
                                case.wings[wing_index].label for case in solver_cases
                                if wing_index < len(case.wings)
                            )
                            color = _wing_color(wing_index)
                            symbol = solver_markers.get(solver, "circle") + reference_fill

                        name = f"{wing_name}, {solver.value}"

                        if wing_index is None:
                            total_traces.append(len(figure.data))
                        else:
                            wing_traces.append(len(figure.data))

                        if len(reference_labels) > 1:
                            name += f" vs. {reference_label}"

                        figure.add_trace(
                            go.Scatter(
                                x=x,
                                y=y,
                                mode="markers",
                                name=name,
                                legendgroup=name,
                                showlegend=(col == 1),
                                marker=dict(symbol=symbol, size=10 if wing_index is not None else 13, color=color),
                                text=text,
                                hovertemplate=(
                                    f"{name}<br>%{{text}}<br>reference = %{{x:.3f}}<br>"
                                    f"simulation = %{{y:.3f}}<extra></extra>"
                                ),
                            ),
                            row=1, col=col
                        )

    shapes = []

    reference_text = ", ".join(reference_labels)

    for col, key in ((1, "thrust"), (2, "side_force")):
        values = all_values[key]

        if not values:
            continue

        low, high = min(values), max(values)
        padding = 0.05 * (high - low) if high > low else 0.1

        low, high = low - padding, high + padding

        axis = "" if col == 1 else str(col)

        shapes.append(dict(
            type="line", xref=f"x{axis}", yref=f"y{axis}", x0=low, y0=low, x1=high, y1=high,
            line=dict(color="gray", dash="dash", width=1.5),
        ))

        figure.update_xaxes(range=[low, high], title_text=f"Reference ({reference_text}) [-]", row=1, col=col)
        figure.update_yaxes(range=[low, high], title_text="Simulation [-]", row=1, col=col)

    # Room for the buttons for the wings and the total
    menu_position = _figure_layout(figure, 1, 2, 0.0, _COMPARISON_SPACING, 3, has_dropdown=True)

    figure.update_layout(
        title_text=(
            f"<b>Comparison with reference data</b>: {', '.join(reference_labels)}<br>"
            "<sub>Mean simulated coefficients against the reference values, for the cases with "
            "reference data. Color: wing (black stars: total), marker: solver (circle: OpenFOAM, "
            "square: Stormflow)<br>Dashed line: equal values"
            + (". Marker fill: reference data (" + ", ".join(
                f"{label}: {_REFERENCE_FILL_NAMES[index % len(_REFERENCE_FILL_NAMES)]}"
                for index, label in enumerate(reference_labels)
            ) + ")" if len(reference_labels) > 1 else "")
            + "</sub>"
        ),
        shapes=shapes,
        hovermode="closest",
    )

    # The legend entries are the wings and the total, so only the entries of the shown kind are kept
    add_data_toggle(
        figure, _WING_TOGGLE_LABELS, [total_traces, wing_traces], x=1.0,
        keep_legend=False, **menu_position
    )

    return figure


def make_detailed_figure(cases: list[ReportCase]) -> Any:
    '''
    Spanwise distributions and force histories for a single case, chosen with a dropdown menu. The
    color shows the wing, with the same colors as in the comparison. The reference values are only given as numbers in the title, as they have no time history or
    spanwise distribution.
    '''
    import plotly.graph_objects as go # type: ignore[import-not-found, import-untyped, unused-ignore]
    from plotly.subplots import make_subplots # type: ignore[import-not-found, import-untyped, unused-ignore]

    figure = make_subplots(
        rows=2,
        cols=2,
        vertical_spacing=_DETAILED_SPACING[0],
        horizontal_spacing=_DETAILED_SPACING[1],
        subplot_titles=(
            "Circulation strength (+ gives thrust)",
            "Effective angle of attack (+ gives thrust)",
            "Thrust coefficient",
            "Side force coefficient",
        )
    )

    colors = _source_colors(cases)
    source_labels = _source_labels(list(colors))

    trace_group_index = []
    labels = []
    titles = []
    shapes = []

    for case_index, case in enumerate(cases):
        for wing_index, wing in enumerate(case.wings):
            line = dict(color=_wing_color(wing_index))

            if case.distributions is not None:
                distribution = case.distributions[wing_index]

                for col, key, unit in ((1, "circulation", "m^2/s"), (2, "aoa_deg", "deg")):
                    figure.add_trace(
                        go.Scatter(
                            x=distribution["height"],
                            y=distribution[key],
                            mode="lines",
                            name=wing.label,
                            legendgroup=f"wing_{wing_index}",
                            showlegend=(col == 1),
                            line=line,
                            hovertemplate=(
                                f"{wing.label}<br>height = %{{x:.2f}} m<br>"
                                f"value = %{{y:.2f}} {unit}<extra></extra>"
                            ),
                        ),
                        row=1, col=col
                    )

                    trace_group_index.append(case_index)

            for col, key, quantity in ((1, "thrust", "C_T"), (2, "side_force", "C_S")):
                figure.add_trace(
                    go.Scatter(
                        x=case.time,
                        y=case.history[key][wing_index],
                        mode="lines",
                        name=wing.label,
                        legendgroup=f"wing_{wing_index}",
                        # In the legend here only if there are no distributions for this case
                        showlegend=(case.distributions is None and col == 1),
                        line=dict(width=1.5, **line),
                        hovertemplate=(
                            f"{wing.label}, {quantity}<br>t = %{{x:.2f}}<br>"
                            f"value = %{{y:.3f}}<extra></extra>"
                        ),
                    ),
                    row=2, col=col
                )

                trace_group_index.append(case_index)

        case_label = _case_label(case, source_labels)

        labels.append(case_label)

        title = f"<b>{case_label}</b>"

        # The setup name is only part of the label when there are several setups
        if case.setup_name is not None and not case_label.startswith(case.setup_name + ","):
            title = f"<b>{case.setup_name}: {case_label}</b>"

        short_labels = [wing.short_label for wing in case.wings]

        title_lines = [
            title,
            "Simulation C_T: " + _values_text(short_labels, list(case.mean["thrust"]), case.total_mean["thrust"]),
            "Simulation C_S: " + _values_text(short_labels, list(case.mean["side_force"]), case.total_mean["side_force"]),
        ]

        for reference in case.references:
            title_lines += [
                f"{reference.label} C_T: "
                + _values_text(short_labels, reference.coefficients["thrust"], reference.total["thrust"]),
                f"{reference.label} C_S: "
                + _values_text(short_labels, reference.coefficients["side_force"], reference.total["side_force"]),
            ]

        wing_text = ", ".join(f"{wing.short_label} = {wing.label}" for wing in case.wings)

        title_lines += [
            f"Distributions at t = {case.distribution_time:.1f}. Target: {case.target_label}",
            wing_text,
            "<i>Dotted lines: target angle of attack and start of the averaging window</i>",
        ]

        titles.append("<br>".join(title_lines))

        case_shapes = [
            # Start of the averaging window in the force histories
            dict(
                type="line", xref=xref, yref=f"{yref} domain",
                x0=case.averaging_start_time, x1=case.averaging_start_time, y0=0.0, y1=1.0,
                line=dict(color="gray", dash="dot", width=1.5),
            )
            for xref, yref in (("x3", "y3"), ("x4", "y4"))
        ]

        targets = sorted(set(
            wing.target_aoa_deg for wing in case.wings if wing.target_aoa_deg is not None
        ))

        for target in targets:
            case_shapes.append(
                dict(
                    type="line", xref="x2 domain", yref="y2",
                    x0=0.0, x1=1.0, y0=target, y1=target,
                    line=dict(color="gray", dash="dot", width=1.5),
                )
            )

        shapes.append(case_shapes)

    history_axis_title = "Time [s] (iteration for steady OpenFOAM)"

    figure.update_xaxes(title_text="Height above the water plane [m]", row=1, col=1)
    figure.update_yaxes(title_text="Γ [m^2/s]", row=1, col=1)
    figure.update_xaxes(title_text="Height above the water plane [m]", row=1, col=2)
    figure.update_yaxes(title_text="Angle of attack [deg]", row=1, col=2)
    figure.update_xaxes(title_text=history_axis_title, row=2, col=1)
    figure.update_yaxes(title_text="C_T [-]", row=2, col=1)
    figure.update_xaxes(title_text=history_axis_title, row=2, col=2)
    figure.update_yaxes(title_text="C_S [-]", row=2, col=2)

    nr_title_lines = max((title.count("<br>") + 1 for title in titles), default=1)

    menu_position = _figure_layout(
        figure, 2, 2, *_DETAILED_SPACING, nr_title_lines, has_dropdown=True
    )

    if len(cases) > 0:
        add_dropdown(
            figure, labels, titles, trace_group_index, shapes=shapes,
            x=0.0, **menu_position,
        )

    return figure


def _report_figures(cases: list[ReportCase], averaging_fraction: float) -> list[Any]:
    '''The overview, the comparison, if there is reference data, and the detailed view'''
    figures = [make_overview_figure(cases, averaging_fraction)]

    comparison = make_comparison_figure(cases)

    if comparison is not None:
        figures.append(comparison)

    figures.append(make_detailed_figure(cases))

    return figures


def sails_on_ship_report(
    results_folder: Path | str,
    output_file: Path | str | None = None,
    averaging_fraction: float = DEFAULT_AVERAGING_FRACTION,
    reference_files: list[Path | str] | None = None,
    combine_wind_velocities: bool = False,
) -> Path:
    '''
    Writes an HTML report of all the result files in the results folder that follow the naming
    convention and contain the simulation builder, compared with the reference data, if any. See
    the module documentation for the content. The reference data is read from the given files, or,
    if not given, from the files in the results folder that end in REFERENCE_FILE_SUFFIX. The report
    is written to the output file, or to "sails_on_ship_report.html" in the results folder if not
    given. Returns the path of the report.

    With combine_wind_velocities, cases that only differ in the true wind velocity are shown as one
    source, i.e., connected with lines in the overview, see read_report_cases. False by default.
    '''
    results_folder = Path(results_folder)

    output_path = (
        results_folder / DEFAULT_REPORT_FILE_NAME if output_file is None else Path(output_file)
    )

    cases = read_report_cases(
        results_folder, averaging_fraction, reference_files, combine_wind_velocities
    )

    if len(cases) == 0:
        raise ValueError(f"No result files for the report in {results_folder}")

    setup_names = _setup_names(cases)

    if len(setup_names) == 1:
        write_html(
            _report_figures(cases, averaging_fraction), output_path, title=REPORT_TITLE,
            max_width=REPORT_MAX_WIDTH
        )
    else:
        # One group of figures for each setup, chosen with a dropdown menu at the top of the page
        groups = [
            (
                _setup_label(setup_name),
                _report_figures(
                    [case for case in cases if case.setup_name == setup_name], averaging_fraction
                ),
            )
            for setup_name in setup_names
        ]

        write_html_with_selector(
            groups, output_path, selector_label="Setup", title=REPORT_TITLE,
            max_width=REPORT_MAX_WIDTH
        )

    return output_path
