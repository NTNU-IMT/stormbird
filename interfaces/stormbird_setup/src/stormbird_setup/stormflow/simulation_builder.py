
from pydantic import Field

from ..base_model import StormbirdSetupBaseModel
from ..spatial_vector import SpatialVector
from ..wind import WindCondition, WindEnvironment
from .grid import GridBuilder
from .geometry import GeometryBuilder
from .pressure_solver import PressureSolverBuilder, ComputePlatform
from .solver_settings import SolverSettings, SlipMirrorInterpolationOrder
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
    pressure_solver: PressureSolverBuilder = Field(default_factory=PressureSolverBuilder)
    solver_settings: SolverSettings = Field(default_factory=SolverSettings)
    slip_velocity_interpolation_order: SlipMirrorInterpolationOrder = (
        SlipMirrorInterpolationOrder.Tricubic
    )
    slip_wall_boundary_override: list[list[bool]] | None = None
    velocity_solver_compute_platform: ComputePlatform = ComputePlatform.CPU
    """
    Where to execute the velocity solver. Independent of where the pressure solver is executed
    (see `compute_platform` in `MultigridSettingsBuilder`), but the fewest transfers between the
    host and the device happen when both are on the same platform.
    """
