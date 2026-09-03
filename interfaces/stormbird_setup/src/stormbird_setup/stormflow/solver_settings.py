"""
Copyright (C) 2024, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
"""

from enum import Enum

from ..base_model import StormbirdSetupBaseModel


class SolverSettings(StormbirdSetupBaseModel):
    nr_inner_iterations: int = 2
    solve_pressure_on_first_iteration: bool = True


class SlipMirrorInterpolationOrder(Enum):
    """
    Interpolation order used to sample the mirrored image point for the slip-wall velocity
    correction specifically. Defaults to 4th order (`Tricubic`); switch to `Trilinear` (2nd order)
    for thin walls.
    """

    Trilinear = "Trilinear"
    Tricubic = "Tricubic"
