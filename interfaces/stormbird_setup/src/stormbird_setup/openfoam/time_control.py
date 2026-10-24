"""
Copyright (C) 2026, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
"""

from ..base_model import StormbirdSetupBaseModel

class SteadyTimeControl(StormbirdSetupBaseModel):
    '''
    Steady state simulation with simpleFoam
    '''
    max_iterations: int = 1000
    """Maximum number of iterations. The simulation stops earlier if the residuals converge."""

class DynamicTimeControl(StormbirdSetupBaseModel):
    '''
    Time dependent simulation with pimpleFoam, with the time step adjusted to the Courant number
    '''
    time_step: float
    """Initial and maximum time step."""
    end_time: float
    max_courant_number: float = 1.0
