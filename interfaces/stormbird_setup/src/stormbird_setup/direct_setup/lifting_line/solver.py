'''
Copyright (C) 2024, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
'''

from ...base_model import StormbirdSetupBaseModel
from .velocity_corrections import VelocityCorrections

from enum import Enum

class FinalCirculationCorrectionMethod(Enum):
    NoCorrection = "NoCorrection"
    IncludeViscousLiftInEffectiveCirculation = "IncludeViscousLiftInEffectiveCirculation"
    FullCorrection = "FullCorrection"

class Linearized(StormbirdSetupBaseModel):
    disable_viscous_corrections: bool = False
    velocity_corrections: VelocityCorrections = VelocityCorrections()
    final_circulation_correction_method: FinalCirculationCorrectionMethod = FinalCirculationCorrectionMethod.FullCorrection


class SimpleIterative(StormbirdSetupBaseModel):
    max_iterations_per_time_step: int
    damping_factor: float
    residual_tolerance_absolute: float = 1e-4
    strength_difference_tolerance: float = 1e-6
    velocity_corrections: VelocityCorrections = VelocityCorrections()
    start_with_linearized_solution: bool = False