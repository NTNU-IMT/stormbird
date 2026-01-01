from ...base_model import StormbirdSetupBaseModel

from ..lifting_line.wake import SymmetryCondition

class LiftingLineCorrectionBuilder(StormbirdSetupBaseModel):
    wake_length_factor: float = 100.0
    symmetry_condition: SymmetryCondition = SymmetryCondition.NoSymmetry
    initialization_time: float | None = None

class EmpiricalCirculationCorrection(StormbirdSetupBaseModel):
    exp_factor: float = 10.0
    overall_correction: float = 1.0
