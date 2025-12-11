
from pystormbird import SimulationResult

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
    def __init__(self, input_string: str) -> None: ...
    def do_step(
        self,
        *,
        time: float, 
        time_step: float, 
        wind_velocity: float, 
        wind_direction: float, 
        ship_velocity: float,
        controller_loading: float
    ) -> SimulationResult: ...
    
    def simulate_condition(
        self,
        *,
        wind_velocity: float,
        wind_direction: float,
        ship_velocity: float,
        controller_loading: float = 1.0,
        time_step: float = 1.0,
        nr_time_steps: int = 1
    ) -> SimulationResult: ...
    
    def simulate_condition_optimal_controller_loading(
        self,
        *,
        wind_velocity: float,
        wind_direction: float,
        ship_velocity: float,
        nr_loadings_to_test: int = 10,
        time_step: float = 1.0,
        nr_time_steps: int = 1
    ) -> SimulationResult: ...
    
    def section_models_internal_state(self) -> list[float]: ...
