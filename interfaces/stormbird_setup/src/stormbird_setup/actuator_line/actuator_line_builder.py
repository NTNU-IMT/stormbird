"""
Copyright (C) 2024, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
"""
import warnings

from pydantic import model_validator

from ..base_model import StormbirdSetupBaseModel
from ..line_force_model import LineForceModelBuilder

from .settings import ProjectionSettings, SolverSettings, SamplingSettings
from .corrections import LiftingLineCorrectionBuilder, TipLossCorrection

from ..controller import ControllerBuilder

class ActuatorLineBuilder(StormbirdSetupBaseModel):
    line_force_model: LineForceModelBuilder
    projection_settings: ProjectionSettings = ProjectionSettings()
    solver_settings: SolverSettings = SolverSettings()
    sampling_settings: SamplingSettings = SamplingSettings()
    write_iterations_full_result: int = 100
    start_time: float = 0
    controller: ControllerBuilder | None = None
    lifting_line_correction: LiftingLineCorrectionBuilder | None = LiftingLineCorrectionBuilder()
    """
    Correction of the sampled velocity, based on a lifting line model. Used by default. Set to `None`
    to turn the correction off.
    """
    tip_loss_correction: TipLossCorrection | None = None
    """
    Empirical tip loss correction of the circulation. Not used by default.
    """

    @model_validator(mode='after')
    def check_dynamic_stall_damping(self) -> "ActuatorLineBuilder":
        '''
        The damping of the circulation acts as an additional time lag, which also affects the
        dynamic response of the forces. Gives a warning if a dynamic stall model is used together
        with a damping factor below one.
        '''
        damping_factor = self.solver_settings.damping_factor

        if self.line_force_model.has_dynamic_stall() and damping_factor < 1.0:
            time_lag = 1.0 / damping_factor if damping_factor > 0.0 else float('inf')

            warnings.warn(
                "The actuator line model has a dynamic stall model, but the damping factor of the "
                f"circulation is {damping_factor}. The damping acts as an additional time lag of "
                f"about {time_lag:.1f} time steps, which also affects the dynamic "
                "response of the forces."
            )

        return self
