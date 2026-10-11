"""
Copyright (C) 2024, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
"""

from enum import Enum

from pydantic import Field

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

    `WallModel` (default) uses a mirror (slip) correction inside the geometries, together with the
    wall shear stress from the log-law, applied as a momentum sink close to the surface, and the
    data immersion close to the sharp edges (see `SharpEdgeSettings`). Requires geometries that are
    at least a few cells thick, and `effective_viscosity` should be the molecular viscosity.

    `DataImmersion` blends the velocity towards zero in a band of a few cells around the surface.
    Robust, also for thin geometries, but the wall shear stress depends on the grid resolution.
    """

    DataImmersion = "DataImmersion"
    WallModel = "WallModel"


class SharpEdgeSettings(StormbirdSetupBaseModel):
    """
    Settings for the data immersion close to the sharp convex edges of the geometries with a mirror
    (slip) correction, which trips the flow separation there. Used with separate settings for the
    no-slip geometries with the wall model (`sharp_edges`), and for the slip geometries
    (`slip_sharp_edges`), both on by default. The edges are taken from the input geometries: the edges of cuboids, the rims of disks
    (unless rounded with a fillet radius larger than half a cell), and the feature edges of
    triangle meshes. The weight that is used for each velocity face is exported as the field
    `sharp_edge_weight`.
    """

    enabled: bool = True
    """
    Whether the data immersion is applied close to the sharp edges.
    """
    min_angle_degrees: float = 30.0
    """
    The smallest angle, in degrees, between the surface normals on each side of an edge of a
    triangle mesh for it to count as sharp. Edges with smaller angles are treated as part of a
    smooth, curved surface.
    """
    inner_radius_cells: float = 1.0
    """
    The distance from the edges, as a number of (the largest) cell lengths, within which the full
    data immersion is used.
    """
    outer_radius_cells: float = 3.0
    """
    The distance from the edges, as a number of (the largest) cell lengths, beyond which only the
    wall model is used. Between the inner and outer radius, the two are blended smoothly. Must be
    larger than `inner_radius_cells`.
    """


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
    no_slip_wall_treatment: NoSlipWallTreatment = NoSlipWallTreatment.WallModel
    """
    How the no-slip geometries are treated: with the wall model (default), which is a slip
    condition together with the wall shear stress from the log-law, or with the data immersion.
    """
    sharp_edges: SharpEdgeSettings = Field(default_factory=SharpEdgeSettings)
    """
    Settings for the data immersion close to the sharp convex edges of the no-slip geometries. Only
    used with the wall model, where it is on by default.
    """
    slip_sharp_edges: SharpEdgeSettings = Field(default_factory=SharpEdgeSettings)
    """
    Settings for the data immersion close to the sharp convex edges of the slip geometries, which
    makes the flow separate at the edges, while the rest of the slip surfaces stay inviscid. On by
    default, as sharp edges are also troublesome from a numerical point of view.
    """
    no_slip_blending_cells: float = 2.0
    """
    The half width of the band where the data immersion blends the velocity towards zero, as a
    number of (the largest) cell lengths. Also used for the wall functions of the turbulence model.
    Only used with the data immersion, including the data immersion close to sharp edges. Values below about one cell can cause oscillations close to
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
