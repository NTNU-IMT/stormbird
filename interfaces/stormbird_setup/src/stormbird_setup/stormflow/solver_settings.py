"""
Copyright (C) 2024, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
"""

from ..base_model import StormbirdSetupBaseModel

# Moved to velocity_solver.py, and re-exported here for existing imports
from .velocity_solver import SlipMirrorInterpolationOrder, NoSlipWallTreatment  # noqa: F401


class SolverSettings(StormbirdSetupBaseModel):
    nr_inner_iterations: int = 2
    solve_pressure_on_first_iteration: bool = True
