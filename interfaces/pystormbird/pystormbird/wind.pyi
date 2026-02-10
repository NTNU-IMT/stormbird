class WindEnvironment:
    def __init__(self, setup_string: str) -> None: ...
    def true_wind_velocity_at_height(
        self, 
        *, 
        wind_velocity: float, 
        wind_direction_coming_from: float,
        height: float
    ) -> float: ...
    
    def true_wind_velocity_vector_at_location(
        self, 
        *, 
        wind_velocity: float,
        wind_direction_coming_from: float,
        location: list[float]
    ) -> list[float]: ...
    
    def apparent_wind_velocity_vector_at_location(
        self,
        *,
        wind_velocity: float,
        wind_direction_coming_from: float,
        location: list[float],
        linear_velocity: list[float]
    ) -> list[float]: ...
    
    def apparent_wind_direction_from_condition_and_linear_velocity(
        self,
        *,
        wind_velocity: float,
        wind_direction_coming_from: float,
        linear_velocity: list[float],
        height: float = 10.0
    ) -> float: ...
    
    def apparent_wind_velocity_vectors_at_ctrl_points_with_corrections_applied(
        self,
        *,
        wind_velocity: float, 
        wind_direction_coming_from: float,
        ctrl_points: list[list[float]],
        linear_velocity: list[float],
        wing_indices: list[list[int]]
    ) -> list[list[float]]: ...