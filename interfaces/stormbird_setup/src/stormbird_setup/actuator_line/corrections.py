"""
Copyright (C) 2024, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
"""

from ..base_model import StormbirdSetupBaseModel

from ..lifting_line.wake import SymmetryCondition

class LiftingLineCorrectionBuilder(StormbirdSetupBaseModel):
    wake_length_factor: float = 100.0
    symmetry_condition: SymmetryCondition = SymmetryCondition.NoSymmetry
    initialization_time: float | None = None
    reference_core_length_factor: float = 100.0
    include_wing_interactions: bool = False
    max_correction_ratio: float | None = None
    """
    Optional limit on the magnitude of the velocity correction at each control point, as a ratio of
    the magnitude of the sampled velocity at the same point. No limit by default.
    """
    virtual_extension_factors: tuple[float, float] | None = None
    """
    Length of a virtual extension of each wing, at the first and the last end, relative to the chord
    length at that end. The extensions are only used in the reference model with the small viscous
    core, and carry the circulation of the outermost line element. They move the tip vortices away
    from the ends of the wing, which can be used to model end plates. No extensions by default.
    """

class EmpiricalCirculationCorrection(StormbirdSetupBaseModel):
    exp_factor: float = 10.0
    overall_correction: float = 1.0
