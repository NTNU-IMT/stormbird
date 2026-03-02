class WindCondition:
    @classmethod
    def new_constant(cls, *, direction_coming_from: float, velocity: float) -> "WindCondition": ...
    
    @classmethod
    def new_power_model(
        cls, 
        *, 
        direction_coming_from: float, 
        reference_velocity: float,
        reference_height: float = 10.0,
        power_factor: float = 0.11111
    ) -> "WindCondition": ...
    
    @classmethod
    def new_logarithmic_model(
        cls,
        *,
        direction_coming_from: float,
        friction_velocity: float,
        surface_roughness: float,
        von_karman_constant: float = 0.41,
        obukhov_length: float = 0.0
    ) -> "WindCondition": ...
    
    def true_wind_velocity_at_height(self, height: float) -> float: ...

class WindEnvironment:
    def __init__(self, setup_string: str) -> None: ...
    
    def true_wind_velocity_at_location(
        self, 
        *, 
        condition: WindCondition,
        location: list[float]
    ) -> float: ...
    
    def true_wind_velocity_vector_at_location(
        self, 
        *, 
        condition: WindCondition,
        location: list[float]
    ) -> list[float]: ...
    
    def apparent_wind_velocity_vector_at_location(
        self,
        *,
        condition: WindCondition,
        location: list[float],
        linear_velocity: list[float]
    ) -> list[float]: ...
    
    def apparent_wind_direction_from_condition_and_linear_velocity(
        self,
        *,
        condition: WindCondition,
        linear_velocity: list[float],
        height: float = 10.0
    ) -> float: ...
    
    def apparent_wind_velocity_vectors_at_ctrl_points_with_corrections_applied(
        self,
        *,
        condition: WindCondition,
        ctrl_points: list[list[float]],
        linear_velocity: list[float],
        wing_indices: list[list[int]]
    ) -> list[list[float]]: ...
