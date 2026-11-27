"""
Copyright (C) 2024, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
"""

from enum import Enum
from typing import Any

from pydantic import model_serializer, model_validator

from ..base_model import StormbirdSetupBaseModel

from ..lifting_line.wake import SymmetryCondition

class StraightWakeBuilder(StormbirdSetupBaseModel):
    """
    Wake model for the lifting line correction, with one straight horseshoe vortex for each line
    element.
    """
    wake_length_factor: float = 100.0
    """
    The length of the wake, relative to the average chord length.
    """
    virtual_extension_factors: tuple[float, float] = (0.0, 0.0)
    """
    Length of a virtual extension of each wing, at the first and the last end, relative to the chord
    length at that end. The extensions are only used in the reference model with the small viscous
    core, and carry the circulation of the outermost line element. They move the tip vortices away
    from the ends of the wing, which can be used to model end plates. No extensions by default.
    """

class StraightPanelWakeBuilder(StormbirdSetupBaseModel):
    """
    Wake model for the lifting line correction, with a straight wake that is split into panels in
    the streamwise direction, so that the reference core length can increase downstream. Requires
    Stormbird 0.14.0 or newer.
    """
    wake_length_factor: float = 100.0
    """
    The length of the wake, relative to the average chord length.
    """
    nr_panels_per_line_element: int = 50
    """
    Number of panels in the streamwise direction behind each line element. Must be at least 2.
    """
    first_panel_relative_length: float = 0.5
    """
    Length of the first panel, relative to the average chord length. The panels get gradually
    longer downstream, so that the total length is given by the wake_length_factor.
    """
    reference_core_evolution_length_factor: float | None = None
    """
    Optional distance along the wake, relative to the average chord length, over which the reference
    core length grows from the small reference value to the viscous core length of the projection
    width. The two wake models are then equal downstream, so that the far wake, e.g., the tip
    vortices from an upstream wing, does not contribute to the correction. Constant reference core
    length if not given.
    """
    far_field_ratio: float = 5.0
    """
    Determines how far away from a panel it is necessary to be before the far field method is used
    to calculate the induced velocity.
    """

class DynamicCorrectionWakeBuilder(StormbirdSetupBaseModel):
    """
    Wake model for the lifting line correction, with a free wake where the shape follows the
    velocity sampled from the CFD flow field at the points of the wake, and the strength of each
    row of panels is the circulation from earlier time steps. A new row of panels is shed in each
    time step, so the length of the wake is approximately the number of panels times the velocity
    times the time step. Requires Stormbird 0.14.0 or newer, and a CFD interface that samples the
    velocity at the wake points.
    """
    nr_panels_per_line_element: int = 100
    """
    Number of panels in the streamwise direction behind each line element. Must be at least 2.
    """
    first_panel_relative_length: float = 0.75
    """
    Length of the first panel, relative to the average chord length. The first panel follows the
    direction of the sampled velocity at the control points.
    """
    last_panel_relative_length: float = 1.0
    """
    Length of the last panel, relative to the average chord length.
    """
    shape_damping_factor: float = 0.0
    """
    Damping of the changes in the shape of the wake, between 0 (no damping) and 1.
    """
    reference_core_evolution_length_factor: float | None = None
    """
    Optional distance along the wake, relative to the average chord length, over which the reference
    core length grows to the viscous core length of the projection width. Same as for the
    StraightPanelWakeBuilder. The length of the dynamic wake depends on the time step, and the
    evolution length should be shorter than the wake.
    """
    far_field_ratio: float = 5.0
    """
    Determines how far away from a panel it is necessary to be before the far field method is used
    to calculate the induced velocity.
    """

CorrectionWakeBuilder = StraightWakeBuilder | StraightPanelWakeBuilder | DynamicCorrectionWakeBuilder

# The name of the variant of the wake enum in Stormbird, for each builder
_WAKE_VARIANTS: dict[type, str] = {
    StraightWakeBuilder: "Straight",
    StraightPanelWakeBuilder: "StraightPanels",
    DynamicCorrectionWakeBuilder: "Dynamic",
}

class LiftingLineCorrectionBuilder(StormbirdSetupBaseModel):
    wake: CorrectionWakeBuilder = StraightWakeBuilder()
    """
    The wake model used to compute the correction. Requires Stormbird 0.14.0 or newer.
    """
    symmetry_condition: SymmetryCondition = SymmetryCondition.NoSymmetry
    initialization_time: float | None = None
    reference_core_length_factor: float = 100.0
    include_wing_interactions: bool = False
    max_correction_ratio: float | None = None
    """
    Optional limit on the magnitude of the velocity correction at each control point, as a ratio of
    the magnitude of the sampled velocity at the same point. No limit by default.
    """

    @model_validator(mode='before')
    @classmethod
    def deserialize_wake(cls, data: Any) -> Any:
        if isinstance(data, dict) and isinstance(data.get('wake'), dict):
            wake_data = data['wake']

            for wake_class, variant in _WAKE_VARIANTS.items():
                if variant in wake_data:
                    data = {**data, 'wake': wake_class(**wake_data[variant])}
                    break

        return data

    @model_serializer
    def ser_model(self) -> dict[str, Any]:
        wake_variant = _WAKE_VARIANTS[type(self.wake)]

        out: dict[str, Any] = {
            "wake": {wake_variant: self.wake.model_dump(exclude_none=True)},
            "symmetry_condition": self.symmetry_condition.value,
            "reference_core_length_factor": self.reference_core_length_factor,
            "include_wing_interactions": self.include_wing_interactions,
        }

        if self.initialization_time is not None:
            out["initialization_time"] = self.initialization_time

        if self.max_correction_ratio is not None:
            out["max_correction_ratio"] = self.max_correction_ratio

        return out

class TipLossApplication(Enum):
    """
    Where the tip loss correction is applied in an actuator line simulation.
    """
    Circulation = "Circulation"
    """
    The correction is applied to the estimated circulation, and therefore affects both the forces on
    the wings and the forces that are projected to the CFD grid.
    """
    ProjectedForces = "ProjectedForces"
    """
    The correction is only applied to the circulation used for the forces that are projected to the
    CFD grid. The forces on the wings are computed from the uncorrected circulation. If a lifting
    line correction is used, the model with the viscous core uses the projected circulation, while
    the reference model uses the uncorrected circulation.
    """

class TipLossCorrection(StormbirdSetupBaseModel):
    """
    Empirical correction of the circulation close to the ends of the wings, with a shape similar to
    the Prandtl tip loss factor.
    """
    exp_factor: float = 10.0
    """
    Factor that determines how fast the correction approaches one away from the ends of the wings.
    """
    maintain_mean_value: bool = True
    """
    If true, the corrected circulation on each wing is scaled so that the mean value over the span
    is the same as before the correction. Requires Stormbird 0.12.0 or newer.
    """
    application: TipLossApplication = TipLossApplication.Circulation
    """
    Where the correction is applied. Requires Stormbird 0.12.0 or newer.
    """
