"""
Copyright (C) 2026, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

Naming convention for the cases in simulations of sails on a ship, so that, e.g., result files and
case folders get the same names in all setups, and the test condition can be read back from a name.

A name consists of parts separated by underscores, in this order:

- the name of the setup, e.g., the sail supplier and the ship, only if given (see check_setup_name)
- the solver, in lower case: "openfoam" or "stormflow"
- "twd" + the true wind direction in degrees, with one decimal
- "twv" + the true wind velocity, with two decimals
- "sv" + the ship velocity, with two decimals
- "constant_wind", only with a constant wind profile. The power law profile is the default.
- "aoa" + the fixed angle of attack in degrees, with one decimal, only if given
- "is" + the fixed internal state, with six significant digits, only if given

For instance: "example_ship_stormflow_twd45.0_twv8.00_sv6.17_aoa10.0_is0.1". The names
contain no file extension.

The values in a name are rounded, so a condition read from a name can differ slightly from the
condition that the name was made from. Names made from the same rounded values are equal.
"""

import math
import re
from typing import NamedTuple

from .conditions import ShipTestCondition, WindProfile
from .simulation_settings import CfdSolver

_SETUP_NAME = r"[A-Za-z0-9][A-Za-z0-9_.-]*"

_NUMBER = r"-?\d+(?:\.\d+)?"

_CASE_NAME_PATTERN = re.compile(
    rf"^(?:(?P<setup_name>{_SETUP_NAME}?)_)?"
    r"(?P<solver>openfoam|stormflow)"
    rf"_twd(?P<twd>{_NUMBER})"
    rf"_twv(?P<twv>{_NUMBER})"
    rf"_sv(?P<sv>{_NUMBER})"
    r"(?P<constant_wind>_constant_wind)?"
    rf"(?:_aoa(?P<aoa>{_NUMBER}))?"
    r"(?:_is(?P<internal_state>-?\d+(?:\.\d+)?(?:e[-+]?\d+)?))?$"
)


class ParsedCaseName(NamedTuple):
    '''The parts of a case name, as read by parse_case_name'''
    setup_name: str | None
    solver: CfdSolver
    condition: ShipTestCondition


def check_setup_name(name: str) -> str:
    '''
    Checks that a setup name can be used in case names, which are also used as folder and file
    names: it must start with a letter or digit, and contain only letters, digits, "_", "-" and ".".
    Returns the name if it is valid.
    '''
    if re.fullmatch(_SETUP_NAME, name) is None:
        raise ValueError(
            f"Invalid setup name: '{name}'. The name must start with a letter or digit, and contain "
            "only letters, digits, '_', '-' and '.'"
        )

    return name


def _solver_name(solver: CfdSolver) -> str:
    return solver.value.lower()


def _solver_from_name(name: str) -> CfdSolver:
    for solver in CfdSolver:
        if _solver_name(solver) == name:
            return solver

    raise ValueError(f"Unknown solver in case name: {name}")


def _fixed(value: float, decimals: int) -> str:
    '''Formats a value with a fixed number of decimals, without negative zero'''
    return f"{round(value, decimals) + 0.0:.{decimals}f}"


def case_name(
    solver: CfdSolver,
    condition: ShipTestCondition,
    setup_name: str | None = None
) -> str:
    '''
    The name of a case, with the setup name, if given, the solver and all values in the test
    condition
    '''
    parts = [] if setup_name is None else [check_setup_name(setup_name)]

    parts += [
        _solver_name(solver),
        "twd" + _fixed(math.degrees(condition.true_wind_direction), 1),
        "twv" + _fixed(condition.true_wind_velocity, 2),
        "sv" + _fixed(condition.ship_velocity, 2),
    ]

    if condition.wind_profile == WindProfile.Constant:
        parts.append("constant_wind")

    if condition.angle_of_attack is not None:
        parts.append("aoa" + _fixed(math.degrees(condition.angle_of_attack), 1))

    if condition.internal_state is not None:
        parts.append(f"is{condition.internal_state + 0.0:g}")

    return "_".join(parts)


def parse_case_name(name: str) -> ParsedCaseName | None:
    '''
    The setup name (None if the name has none), the solver and the test condition from a name made
    by case_name, with the values as rounded in the name. A file extension, e.g., ".json", is
    removed first. Returns None if the name does not follow the naming convention.
    '''
    stem = re.sub(r"\.[A-Za-z]+$", "", name)

    match = _CASE_NAME_PATTERN.match(stem)

    if match is None:
        return None

    aoa = match.group("aoa")
    internal_state = match.group("internal_state")

    condition = ShipTestCondition(
        true_wind_direction = math.radians(float(match.group("twd"))),
        true_wind_velocity = float(match.group("twv")),
        ship_velocity = float(match.group("sv")),
        wind_profile = (
            WindProfile.Constant if match.group("constant_wind") else WindProfile.PowerLaw
        ),
        angle_of_attack = None if aoa is None else math.radians(float(aoa)),
        internal_state = None if internal_state is None else float(internal_state),
    )

    return ParsedCaseName(
        setup_name = match.group("setup_name"),
        solver = _solver_from_name(match.group("solver")),
        condition = condition,
    )
