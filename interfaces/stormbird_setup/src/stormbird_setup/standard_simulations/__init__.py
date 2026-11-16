"""
Copyright (C) 2026, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

Standardized setups of typical simulations, so that the same logic for, e.g., the coordinate
system, the wind and the sail settings can be reused across many cases.
"""

from .actuator_line_runs import (
    run_openfoam_simulation,
    run_stormflow_simulation,
    full_result_as_dict,
    ACTUATOR_LINE_FORCE_DENSITY,
)

__all__ = [
    "run_openfoam_simulation",
    "run_stormflow_simulation",
    "full_result_as_dict",
    "ACTUATOR_LINE_FORCE_DENSITY",
]
