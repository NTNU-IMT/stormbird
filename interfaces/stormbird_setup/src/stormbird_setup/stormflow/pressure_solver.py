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
    """
    How the coarsest multigrid level's Poisson equation is solved at the bottom of each V-cycle.

    `Exact` solves it with Gaussian elimination on a dense matrix, which can be very slow, and use a
    lot of memory, if the coarsest level has many cells. `Jacobi` solves it approximately with extra
    Jacobi iterations, and is the default.
    """

    Exact = "Exact"
    Jacobi = "Jacobi"


class ZeroGradientOnWalls(Enum):
    """
    Which walls get a zero-gradient (Neumann) boundary condition on the pressure, enforced by
    mirroring the pressure across the wall surface for the cells just inside each geometry. This is
    independent of how the walls are represented in the velocity solver. Without this condition, the
    walls only affect the pressure through the velocity field.
    """

    NotUsed = "NotUsed"
    SlipWallsOnly = "SlipWallsOnly"
    NoSlipWallsOnly = "NoSlipWallsOnly"
    AllWalls = "AllWalls"


class ZeroGradientInterpolationOrder(Enum):
    """
    Interpolation order used to sample the pressure at the mirrored image point for the
    zero-gradient condition on walls specifically.
    """

    Trilinear = "Trilinear"
    Tricubic = "Tricubic"


class MultigridSettingsBuilder(StormbirdSetupBaseModel):
    nr_v_cycles: int = 2
    nr_smooth_iterations: int = 4
    compute_residual_after_solve: bool = False
    compute_platform: ComputePlatform = ComputePlatform.CPU
    coarsest_level_solver: CoarsestLevelSolver = CoarsestLevelSolver.Jacobi
    zero_gradient_on_walls: ZeroGradientOnWalls = ZeroGradientOnWalls.NotUsed
    """
    Experimental: which walls get a zero-gradient (Neumann) boundary condition on the pressure,
    independently of how the walls are represented in the velocity solver. Not used by default.
    Works on both `ComputePlatform.CPU` and `ComputePlatform.GPU`.
    """
    zero_gradient_interpolation_order: ZeroGradientInterpolationOrder = (
        ZeroGradientInterpolationOrder.Trilinear
    )
    """
    Interpolation order for the zero-gradient condition on walls specifically (ignored when
    `zero_gradient_on_walls` is `NotUsed`).
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
