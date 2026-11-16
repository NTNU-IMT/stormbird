"""
Copyright (C) 2026, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

Report of the results from simulations of sails on a ship, as an HTML file with two interactive
figures, made from a folder with result files. Requires plotly, which is an optional dependency
(install with the "report" extra).

The result files must be named with the naming convention in naming.py, and contain the simulation
builder that was used, as stored by the run functions in standard_simulations. Other files are
skipped. The results can be from several setups, solvers and test conditions.

The cases are grouped in sources, where a source is everything in the case name except the true
wind direction, i.e., the setup, the solver, the wind and ship velocities, the wind profile and the
fixed sail settings. Each source has its own color, and each wing its own line style.

1) Overview, as a function of the apparent wind direction at mid-span:
   - the mean thrust coefficient (force along the forward direction of the ship) of each wing
   - the mean side force coefficient (force towards port) of each wing
   - the effective angle of attack of each wing, i.e., the angle used in the sectional model,
     including the induced velocities, as the median over the wing and the mean over the full
     results in the averaging window
2) Detailed view of a single case, chosen from a dropdown menu:
   - the spanwise distributions of the circulation strength and the effective angle of attack, from
     the last full result, together with the target angle: the fixed geometric angle of attack, or
     the angle of attack set point of the controller
   - the time history of the thrust and the side force coefficients, with a vertical line at the
     start of the averaging window, as a check of the convergence

All angles of attack and circulation strengths are shown with the sign that gives thrust, i.e.,
positive values give lift with a positive thrust component, as in the test conditions. Note that the
effective angle of attack can be negative for sails with a large lift at zero angle of attack, such as
suction sails, as the induced velocities then are large.

The force coefficients of each wing are made non-dimensional with the dynamic pressure of the
undisturbed apparent wind at the mid-span of the wing, and the projected area of the wing. The
forces in the results are forces per unit density, so the density in the dynamic pressure is
ACTUATOR_LINE_FORCE_DENSITY. The means are taken over the last part of the simulated time, given by
the averaging fraction. For the steady OpenFOAM simulations, the time is the iteration number.
"""

import math
import warnings
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import numpy as np

from ...spatial_vector import SpatialVector
from ...wind import WindCondition, WindEnvironment
from ...actuator_line import ActuatorLineBuilder
from ...line_force_model import WingBuilder
from ...openfoam.results import ActuatorLineResults
from ..actuator_line_runs import ACTUATOR_LINE_FORCE_DENSITY
from ..plot_utils import add_dropdown, averaging_start_index, write_html

from .conditions import ShipTestCondition, WindProfile
from .naming import parse_case_name
from .simulation_settings import CfdSolver
from .sails_on_ship import thrust_sign, _mid_span_point, _non_virtual_end_points

DEFAULT_AVERAGING_FRACTION = 0.25
"""Fraction of the simulated time, counted from the end, that the mean values are computed over"""

DEFAULT_REPORT_FILE_NAME = "sails_on_ship_report.html"

# Line and marker styles for each wing
_WING_DASH = ["solid", "dash", "dot", "dashdot", "longdash", "longdashdot"]
_WING_MARKER = ["circle", "diamond", "square", "triangle-up", "cross", "x"]

# Layout of the detailed figure, in pixels. From the top: the dropdown menu, the title with the
# details of the selected case, and the subplots. The values are chosen so that these do not overlap.
_DETAILED_HEIGHT = 1100
_DETAILED_TOP_MARGIN = 185
_DETAILED_BOTTOM_MARGIN = 80
_DROPDOWN_TOP = 10
_DETAILED_TITLE_TOP = 60

_OVERVIEW_HEIGHT = 1050
_OVERVIEW_TOP_MARGIN = 130


@dataclass
class ReportWing:
    '''A wing in a case, with the values needed in the report'''
    label: str
    area: float
    thrust_sign: float
    """Sign that the angles of attack and circulation strengths are multiplied with"""
    apparent_wind_velocity: float
    """Undisturbed apparent wind velocity at mid-span"""
    target_aoa_deg: float | None
    """The fixed geometric angle of attack or the controller set point, with thrust positive"""
    real_point_indices: list[int]
    """Indices of the control points on the wing itself, i.e., not on virtual segments"""


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
    effective_aoa_deg: list[float]
    averaging_start_time: float
    distributions: list[dict[str, np.ndarray]] | None
    """Spanwise "height", "circulation" and "aoa_deg" of each wing, from the last full result"""
    distribution_time: float

    @property
    def true_wind_direction_deg(self) -> float:
        return round(math.degrees(self.condition.true_wind_direction), 1)

    def source_key(self) -> tuple[Any, ...]:
        '''Everything in the case name except the true wind direction'''
        condition = self.condition

        return (
            self.setup_name,
            self.solver,
            condition.true_wind_velocity,
            condition.ship_velocity,
            condition.wind_profile,
            None if condition.angle_of_attack is None else round(math.degrees(condition.angle_of_attack), 1),
            condition.internal_state,
        )


def _source_sort_key(source: tuple[Any, ...]) -> tuple[Any, ...]:
    '''Sorts by setup and solver, with the controller first, and then the fixed angles'''
    setup_name, solver, true_wind_velocity, ship_velocity, wind_profile, aoa_deg, internal_state = source

    return (
        setup_name or "",
        list(CfdSolver).index(solver),
        true_wind_velocity,
        ship_velocity,
        list(WindProfile).index(wind_profile),
        aoa_deg is not None,
        aoa_deg or 0.0,
        internal_state is not None,
        internal_state or 0.0,
    )


def _source_labels(sources: list[tuple[Any, ...]]) -> dict[tuple[Any, ...], str]:
    '''
    Labels for the sources, with the solver and the sail settings, and the other parts only if they
    differ between the sources
    '''
    varying = [len(set(values)) > 1 for values in zip(*sources)] if sources else []

    labels = {}

    for source in sources:
        setup_name, solver, true_wind_velocity, ship_velocity, wind_profile, aoa_deg, internal_state = source

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

        control = "controller" if aoa_deg is None else f"fixed AoA {aoa_deg:.1f} deg"

        if internal_state is not None:
            control += f", IS {internal_state:g}"

        parts.append(control)

        labels[source] = ", ".join(parts)

    return labels


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
    if condition.angle_of_attack is not None:
        return math.degrees(condition.angle_of_attack)

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
        effective_aoa_deg = effective_aoa_deg,
        averaging_start_time = averaging_start_time,
        distributions = distributions,
        distribution_time = distribution_time,
    )


def read_report_cases(
    results_folder: Path | str,
    averaging_fraction: float = DEFAULT_AVERAGING_FRACTION
) -> list[ReportCase]:
    '''
    All cases in the results folder that can be used in the report, sorted by source and then by
    apparent wind direction
    '''
    cases = [
        case
        for file_path in sorted(Path(results_folder).glob("*.json"))
        if (case := read_report_case(file_path, averaging_fraction)) is not None
    ]

    cases.sort(
        key=lambda case: (_source_sort_key(case.source_key()), case.apparent_wind_direction_deg)
    )

    return cases


def _source_colors(cases: list[ReportCase]) -> dict[tuple[Any, ...], str]:
    import plotly.colors # type: ignore[import-not-found, import-untyped, unused-ignore]

    palette = plotly.colors.qualitative.Plotly

    sources = sorted(set(case.source_key() for case in cases), key=_source_sort_key)

    return {source: palette[index % len(palette)] for index, source in enumerate(sources)}


def _case_label(case: ReportCase, source_labels: dict[tuple[Any, ...], str]) -> str:
    return (
        f"{source_labels[case.source_key()]}, TWD {case.true_wind_direction_deg:.0f} deg "
        f"(AWD {case.apparent_wind_direction_deg:.1f} deg)"
    )


def _setup_names_text(cases: list[ReportCase]) -> str:
    names = sorted(set(case.setup_name for case in cases if case.setup_name is not None))

    return ", ".join(names) if names else "sails on ship"


def make_overview_figure(
    cases: list[ReportCase],
    averaging_fraction: float = DEFAULT_AVERAGING_FRACTION
) -> Any:
    '''Mean values as a function of the apparent wind direction, for all cases'''
    import plotly.graph_objects as go # type: ignore[import-not-found, import-untyped, unused-ignore]
    from plotly.subplots import make_subplots # type: ignore[import-not-found, import-untyped, unused-ignore]

    figure = make_subplots(
        rows=3,
        cols=1,
        shared_xaxes=True,
        vertical_spacing=0.07,
        subplot_titles=(
            "Mean thrust coefficient (+ towards the bow)",
            "Mean side force coefficient (+ towards port)",
            "Effective angle of attack (median over the wing, + gives thrust)",
        )
    )

    colors = _source_colors(cases)
    source_labels = _source_labels(list(colors))

    for source, color in colors.items():
        source_cases = [case for case in cases if case.source_key() == source]

        x = [case.apparent_wind_direction_deg for case in source_cases]

        hover_text = [
            f"{source_labels[source]}, TWD {case.true_wind_direction_deg:.1f} deg"
            for case in source_cases
        ]

        nr_wings = max(len(case.wings) for case in source_cases)

        for wing_index in range(nr_wings):
            wing_cases = [case for case in source_cases if wing_index < len(case.wings)]

            name = f"{source_labels[source]}, {wing_cases[0].wings[wing_index].label}"

            rows = (
                (1, [case.mean["thrust"][wing_index] for case in wing_cases]),
                (2, [case.mean["side_force"][wing_index] for case in wing_cases]),
                (3, [case.effective_aoa_deg[wing_index] for case in wing_cases]),
            )

            for row, y in rows:
                figure.add_trace(
                    go.Scatter(
                        x=[case.apparent_wind_direction_deg for case in wing_cases],
                        y=y,
                        mode="lines+markers",
                        name=name,
                        # Grouped per source, so that one click in the legend toggles the source
                        legendgroup=source_labels[source],
                        showlegend=(row == 1),
                        line=dict(color=color, dash=_WING_DASH[wing_index % len(_WING_DASH)]),
                        marker=dict(symbol=_WING_MARKER[wing_index % len(_WING_MARKER)], size=8),
                        text=hover_text if len(wing_cases) == len(x) else None,
                    ),
                    row=row, col=1
                )

    figure.update_yaxes(title_text="C_T [-]", row=1, col=1)
    figure.update_yaxes(title_text="C_S [-]", row=2, col=1)
    figure.update_yaxes(title_text="Angle of attack [deg]", row=3, col=1)
    figure.update_xaxes(
        title_text="Apparent wind direction at mid-span [deg] (0 = head wind, + from starboard)",
        row=3, col=1
    )

    figure.update_layout(
        height=_OVERVIEW_HEIGHT,
        margin=dict(t=_OVERVIEW_TOP_MARGIN),
        title=dict(
            text=(
                f"Overview: {_setup_names_text(cases)}<br>"
                f"<sub>Means over the last {averaging_fraction:.0%} of the simulated time. "
                "Coefficients relative to the apparent wind at mid-span and the projected area. "
                "Color: source, line style: wing</sub>"
            ),
            x=0.0,
            xref="paper",
            xanchor="left",
        ),
        hovermode="x unified",
        template="plotly_white",
        legend=dict(groupclick="togglegroup"),
    )

    return figure


def make_detailed_figure(cases: list[ReportCase]) -> Any:
    '''
    Spanwise distributions and force histories for a single case, chosen with a dropdown menu. The
    color is the color of the source in the overview, and the line style shows the wing.
    '''
    import plotly.graph_objects as go # type: ignore[import-not-found, import-untyped, unused-ignore]
    from plotly.subplots import make_subplots # type: ignore[import-not-found, import-untyped, unused-ignore]

    figure = make_subplots(
        rows=2,
        cols=2,
        vertical_spacing=0.16,
        horizontal_spacing=0.09,
        subplot_titles=(
            "Circulation strength (+ gives thrust)",
            "Effective angle of attack (dotted: target)",
            "Thrust coefficient (dotted: averaging start)",
            "Side force coefficient (dotted: averaging start)",
        )
    )

    colors = _source_colors(cases)
    source_labels = _source_labels(list(colors))

    trace_group_index = []
    labels = []
    titles = []
    shapes = []

    for case_index, case in enumerate(cases):
        color = colors[case.source_key()]

        for wing_index, wing in enumerate(case.wings):
            line = dict(color=color, dash=_WING_DASH[wing_index % len(_WING_DASH)])
            marker = dict(size=5, symbol=_WING_MARKER[wing_index % len(_WING_MARKER)])

            if case.distributions is not None:
                distribution = case.distributions[wing_index]

                for col, key, unit in ((1, "circulation", "m^2/s"), (2, "aoa_deg", "deg")):
                    figure.add_trace(
                        go.Scatter(
                            x=distribution["height"],
                            y=distribution[key],
                            mode="lines+markers",
                            name=wing.label,
                            legendgroup=f"wing_{wing_index}",
                            showlegend=(col == 1),
                            line=line,
                            marker=marker,
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

        labels.append(_case_label(case, source_labels))

        thrust_text = ", ".join(
            f"{wing.label} {value:.3f}" for wing, value in zip(case.wings, case.mean["thrust"])
        )
        side_force_text = ", ".join(
            f"{wing.label} {value:.3f}" for wing, value in zip(case.wings, case.mean["side_force"])
        )

        case_label = _case_label(case, source_labels)

        title = f"<b>{case_label}</b>"

        # The setup name is only part of the label when there are several setups
        if case.setup_name is not None and not case_label.startswith(case.setup_name + ","):
            title = f"<b>{case.setup_name}: {case_label}</b>"

        titles.append(
            f"{title}<br>"
            f"Mean C_T: {thrust_text}<br>"
            f"Mean C_S: {side_force_text}<br>"
            f"Distributions at t = {case.distribution_time:.1f}. Target: {case.target_label}"
        )

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

    plot_height = _DETAILED_HEIGHT - _DETAILED_TOP_MARGIN - _DETAILED_BOTTOM_MARGIN

    figure.update_layout(
        height=_DETAILED_HEIGHT,
        template="plotly_white",
        margin=dict(t=_DETAILED_TOP_MARGIN, b=_DETAILED_BOTTOM_MARGIN),
        # The title is placed below the dropdown menu, in pixels from the top of the figure
        title=dict(
            x=0.0,
            xref="paper",
            xanchor="left",
            y=1.0 - _DETAILED_TITLE_TOP / _DETAILED_HEIGHT,
            yref="container",
            yanchor="top",
            font=dict(size=14),
        ),
    )

    if len(cases) > 0:
        add_dropdown(
            figure, labels, titles, trace_group_index, shapes=shapes,
            x=0.0,
            y=1.0 + (_DETAILED_TOP_MARGIN - _DROPDOWN_TOP) / plot_height,
            yanchor="top",
        )

    return figure


def sails_on_ship_report(
    results_folder: Path | str,
    output_file: Path | str | None = None,
    averaging_fraction: float = DEFAULT_AVERAGING_FRACTION
) -> Path:
    '''
    Writes an HTML report of all the result files in the results folder that follow the naming
    convention and contain the simulation builder. See the module documentation for the content.
    The report is written to the output file, or to "sails_on_ship_report.html" in the results
    folder if not given. Returns the path of the report.
    '''
    results_folder = Path(results_folder)

    output_path = (
        results_folder / DEFAULT_REPORT_FILE_NAME if output_file is None else Path(output_file)
    )

    cases = read_report_cases(results_folder, averaging_fraction)

    if len(cases) == 0:
        raise ValueError(f"No result files for the report in {results_folder}")

    write_html(
        [make_overview_figure(cases, averaging_fraction), make_detailed_figure(cases)],
        output_path
    )

    return output_path
