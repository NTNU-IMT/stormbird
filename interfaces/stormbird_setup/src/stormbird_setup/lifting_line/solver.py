

from ..base_model import StormbirdSetupBaseModel

class Linearized(StormbirdSetupBaseModel):
    disable_viscous_corrections: bool = False

class SimpleIterative(StormbirdSetupBaseModel):
    max_iterations_per_time_step: int
    damping_factor: float
    residual_tolerance_absolute: float = 1e-4
    strength_difference_tolerance: float = 1e-6