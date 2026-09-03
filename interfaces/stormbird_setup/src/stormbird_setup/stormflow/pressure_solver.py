"""
Copyright (C) 2024, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
"""

from typing import Any
from enum import Enum

from pydantic import model_serializer, model_validator

from ..base_model import StormbirdSetupBaseModel


class ComputePlatform(Enum):
    CPU = "CPU"
    GPU = "GPU"


class CoarsestLevelSolver(Enum):
    """How the coarsest multigrid level's Poisson equation is solved at the bottom of each V-cycle."""

    Exact = "Exact"
    Jacobi = "Jacobi"


class SlipPressureInterpolationOrder(Enum):
    """
    Interpolation order used to sample the mirrored image point for the slip-wall pressure
    correction specifically.
    """

    Trilinear = "Trilinear"
    Tricubic = "Tricubic"


class MultigridSettingsBuilder(StormbirdSetupBaseModel):
    nr_v_cycles: int = 2
    nr_smooth_iterations: int = 4
    compute_residual_after_solve: bool = False
    compute_platform: ComputePlatform = ComputePlatform.CPU
    coarsest_level_solver: CoarsestLevelSolver = CoarsestLevelSolver.Exact
    enable_slip_pressure_correction: bool = False
    """
    Experimental: applies a pressure zero-gradient (Neumann) correction near slip walls during each
    V-cycle. Off by default; only has an effect on `ComputePlatform.CPU`.
    """
    slip_pressure_interpolation_order: SlipPressureInterpolationOrder = (
        SlipPressureInterpolationOrder.Tricubic
    )
    """
    Interpolation order for the slip-wall pressure correction specifically (ignored when
    `enable_slip_pressure_correction` is false).
    """


class PressureSolverBuilder(StormbirdSetupBaseModel):
    """
    Wrapper representing the Rust `PressureSolverBuilder` enum. Serializes into the externally-tagged
    form expected by serde, e.g. `{"Multigrid": {...}}`.
    """

    settings: MultigridSettingsBuilder = MultigridSettingsBuilder()

    @classmethod
    def new_multigrid(cls, settings: MultigridSettingsBuilder) -> "PressureSolverBuilder":
        return cls(settings=settings)

    @model_validator(mode="before")
    @classmethod
    def _deserialize_from_rust_enum(cls, data: Any) -> Any:
        # Already in Python/Pydantic form
        if isinstance(data, dict) and "settings" in data:
            return data

        # Rust externally-tagged enum form, e.g. {"Multigrid": {...}}
        if isinstance(data, dict) and "Multigrid" in data:
            return {"settings": MultigridSettingsBuilder.model_validate(data["Multigrid"])}

        return data

    @model_serializer
    def _serialize_to_rust_enum(self) -> dict[str, Any]:
        return {"Multigrid": self.settings.model_dump(exclude_none=True, mode="json")}
