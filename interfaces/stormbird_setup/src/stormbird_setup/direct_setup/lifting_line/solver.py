'''
Copyright (C) 2024, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
'''

from ...base_model import StormbirdSetupBaseModel
from .velocity_corrections import VelocityCorrections

from enum import Enum

class InducedVelocityCorrectionMethod(Enum):
    NoCorrection = "NoCorrection"
    FullCorrection = "FullCorrection"

class Linearized(StormbirdSetupBaseModel):
    velocity_corrections: VelocityCorrections = VelocityCorrections()
    disable_viscous_corrections: bool = False
    induced_velocity_correction_method: InducedVelocityCorrectionMethod = InducedVelocityCorrectionMethod.FullCorrection


class SimpleIterative(StormbirdSetupBaseModel):
    max_iterations_per_time_step: int = 20
    damping_factor: float = 0.1
    residual_tolerance_absolute: float = 1e-4
    strength_difference_tolerance: float = 1e-6
    velocity_corrections: VelocityCorrections = VelocityCorrections()
    start_with_linearized_solution: bool = False