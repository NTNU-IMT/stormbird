"""
Copyright (C) 2024, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
"""

from enum import Enum

from ..base_model import StormbirdSetupBaseModel
from .pressure_solver import ComputePlatform


class SlipMirrorInterpolationOrder(Enum):
    """
    Interpolation order used to sample the mirrored image point for the velocity mirror correction
    (the slip geometries, and the no-slip geometries with the wall model). Defaults to 2nd order
    (`Trilinear`), which can never amplify oscillatory errors. `Tricubic` (4th order) is more
    accurate for smooth, thick geometries.
    """

    Trilinear = "Trilinear"
    Tricubic = "Tricubic"


class NoSlipWallTreatment(Enum):
    """
    How the velocity solver treats the no-slip geometries.

    `DataImmersion` (default) blends the velocity towards zero in a band of a few cells around the
    surface. Robust, also for thin geometries, but the wall shear stress depends on the grid
    resolution.

    `WallModel` uses a mirror (slip) correction inside the geometries, together with the wall shear
    stress from the log-law, applied as a momentum sink close to the surface. Requires geometries
    that are at least a few cells thick, and `effective_viscosity` should be the molecular
    viscosity.
    """

    DataImmersion = "DataImmersion"
    WallModel = "WallModel"


class VelocitySolverBuilder(StormbirdSetupBaseModel):
    """
    Settings for the velocity solver
    """

    compute_platform: ComputePlatform = ComputePlatform.CPU
    """
    Where to execute the velocity solver. Independent of where the pressure solver is executed
    (see `compute_platform` in `MultigridSettingsBuilder`), but the fewest transfers between the
    host and the device happen when both are on the same platform. The turbulence solver, if used,
    always runs on the same platform as the velocity solver.
    """
    no_slip_wall_treatment: NoSlipWallTreatment = NoSlipWallTreatment.DataImmersion
    """
    How the no-slip geometries are treated: with the data immersion (default), or with the wall
    model, which is a slip condition together with the wall shear stress from the log-law.
    """
    no_slip_blending_cells: float = 2.0
    """
    The half width of the band where the data immersion blends the velocity towards zero, as a
    number of (the largest) cell lengths. Also used for the wall functions of the turbulence model.
    Only used with the data immersion. Values below about one cell can cause oscillations close to
    the surfaces.
    """
    mirror_interpolation_order: SlipMirrorInterpolationOrder = (
        SlipMirrorInterpolationOrder.Trilinear
    )
    """
    Interpolation order for the mirror correction of the velocity, which is used at the slip
    geometries, and at the no-slip geometries with the wall model.
    """
    max_velocity_factor: float | None = None
    """
    Optional limiter on the velocity: each velocity component is clipped to at most this factor
    times the largest inlet velocity magnitude, both before and after the pressure projection. Not
    used by default. This is not physically correct, but a practical safeguard against a few cells
    with very large velocities, for instance at sharp convex edges with the wall model, which would
    otherwise limit the time step for the whole simulation. Monitor the number of clipped values
    with `Simulation.nr_limited_velocity_values()`: if it stays large, the limiter is hiding a real
    problem.
    """
