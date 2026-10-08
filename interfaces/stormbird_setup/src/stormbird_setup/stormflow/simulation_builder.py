
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
    slip_wall_boundary_override: list[list[bool]] | None = None
    turbulence: TurbulenceSolverBuilder | None = None
    """
    Optional RANS turbulence model. The turbulence solver always runs on the same platform as the
    velocity solver. When a turbulence model is used, `effective_viscosity` is the molecular
    viscosity.
    """
