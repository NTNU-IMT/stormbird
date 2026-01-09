'''
Copyright (C) 2024, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
'''

from ...base_model import StormbirdSetupBaseModel
from ..line_force_model import LineForceModelBuilder

from .settings import ProjectionSettings, SolverSettings, SamplingSettings
from .corrections import LiftingLineCorrectionBuilder, EmpiricalCirculationCorrection

from ..controller import ControllerBuilder

class ActuatorLineBuilder(StormbirdSetupBaseModel):
    line_force_model: LineForceModelBuilder
    projection_settings: ProjectionSettings = ProjectionSettings()
    solver_settings: SolverSettings = SolverSettings()
    sampling_settings: SamplingSettings = SamplingSettings()
    write_iterations_full_result: int = 100
    start_time: float = 0
    controller: ControllerBuilder | None = None
    lifting_line_correction: LiftingLineCorrectionBuilder | None = None
    empirical_circulation_correction: EmpiricalCirculationCorrection | None = None
