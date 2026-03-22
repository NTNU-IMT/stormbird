
from pystormbird import SimulationResult
from .line_force_model import LineForceModel
from .wind import WindCondition, WindEnvironment

class Simulation:
    def __init__(self, input_string: str) -> None: ...
    def get_freestream_velocity_points(self) -> list[list[float]]: ...
    def set_local_wing_angles(self, angles: list[float]): ...
    def do_step(
        self, 
        *,
        time: float, 
        time_step: float, 
        freestream_velocity: list[list[float]]
    ) -> SimulationResult: ...
    
class CompleteSailModel:
    def __init__(self, setup_string: str) -> None: ...
    
    def apply_controller(
        self,
        *,
        time: float, 
        time_step: float, 
        wind_condition: WindCondition, 
        ship_velocity: float,
        controller_loading: float
    ) -> None: ...
    
    def do_step(
        self,
        *,
        time: float, 
        time_step: float, 
        wind_condition: WindCondition, 
        ship_velocity: float
    ) -> SimulationResult: ...
    
    def do_multiple_steps(
        self,
        *,
        end_time: float,
        time_step: float,
        wind_condition: WindCondition,
        ship_velocity: float
    ) -> list[SimulationResult]: ...
    
    def section_models_internal_state(self) -> list[float]: ...
    def local_wing_angles(self) -> list[float]: ...
    def set_local_wing_angles(self, local_wing_angles: list[float]) -> None: ...
    def set_section_models_internal_state(self, internal_state: list[float]) -> None: ...
    
    def get_wind_environment(self) -> WindEnvironment: ...
