
from enum import Enum

from pydantic import Field

from ..base_model import StormbirdSetupBaseModel
from ..spatial_vector import SpatialVector
from ..wind import WindCondition, WindEnvironment
from .grid import GridBuilder
from .geometry import GeometryBuilder
from .pressure_solver import PressureSolverBuilder
from .velocity_solver import VelocitySolverBuilder
from .solver_settings import SolverSettings
from .turbulence import TurbulenceSolverBuilder
from ..actuator_line import ActuatorLineBuilder


class BoundaryConditionType(Enum):
    """
    How the boundary conditions are assigned to the faces of the domain. The pressure and the
    turbulence boundary conditions follow the velocity boundary conditions.

    `WindInletOutlet` (default) is for wind propulsion devices in the open air. The face at the
    bottom of the domain, relative to the up direction of the wind environment, is the ground, which
    is a slip wall. The face at the top is open, with a zero gradient condition on the velocity and
    a zero pressure. All the remaining faces are inlet/outlet boundaries.

    `WindTunnel` is for reproducing wind tunnel experiments. The start of the x axis is the inlet,
    and the end of the x axis is an outlet, with a zero gradient condition on the velocity and a
    zero pressure. All the faces along the y and z axes are slip walls. Requires the inlet velocity
    to be along the positive x axis, and the up direction to be along the y or z axis. The inlet
    profile is given by the `wind_condition`, as for `WindInletOutlet`, so it can, for instance,
    be constant or follow an atmospheric boundary layer.
    """

    WindInletOutlet = "WindInletOutlet"
    WindTunnel = "WindTunnel"


class SimulationBuilder(StormbirdSetupBaseModel):
    grid: GridBuilder
    wind_condition: WindCondition
    linear_velocity: SpatialVector
    actuator_line: ActuatorLineBuilder | None = None
    geometries: list[GeometryBuilder] = Field(default_factory=list)
    slip_geometries: list[GeometryBuilder] = Field(default_factory=list)
    effective_viscosity: float = 0.0001
    wind_environment: WindEnvironment = Field(default_factory=WindEnvironment)
    velocity_solver: VelocitySolverBuilder = Field(default_factory=VelocitySolverBuilder)
    """Settings for the velocity solver"""
    pressure_solver: PressureSolverBuilder = Field(default_factory=PressureSolverBuilder)
    """Settings for the pressure solver"""
    solver_settings: SolverSettings = Field(default_factory=SolverSettings)
    boundary_condition_type: BoundaryConditionType = BoundaryConditionType.WindInletOutlet
    """How the boundary conditions are assigned to the faces of the domain"""
    turbulence: TurbulenceSolverBuilder | None = None
    """
    Optional RANS turbulence model. The turbulence solver always runs on the same platform as the
    velocity solver. When a turbulence model is used, `effective_viscosity` is the molecular
    viscosity.
    """
