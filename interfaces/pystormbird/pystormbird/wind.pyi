"""Type stubs for pystormbird.wind."""


class WindCondition:
    @classmethod
    def new_constant(
        cls, *, direction_coming_from: float, velocity: float
    ) -> "WindCondition": ...

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
        obukhov_length: float = 0.0
    ) -> "WindCondition": ...

    def set_parallel_gust_from_json_string(self, gust_string: str) -> None: ...
    def set_perpendicular_gust_from_json_string(self, gust_string: str) -> None: ...
    def set_vertical_gust_from_json_string(self, gust_string: str) -> None: ...

    def steady_true_wind_velocity_at_height(self, height: float) -> float: ...
    def unsteady_parallel_true_wind_velocity_at_height(
        self, height: float, time: float
    ) -> float: ...
    def unsteady_perpendicular_true_wind_velocity(self, time: float) -> float: ...
    def unsteady_vertical_true_wind_velocity(self, time: float) -> float: ...
    def businger_dyer_unscaled_correction(self, height: float) -> float: ...

    @property
    def von_karman_constant(self) -> float: ...
    @property
    def stable_coefficient(self) -> float: ...
    @property
    def unstable_coefficient(self) -> float: ...

    @property
    def friction_velocity(self) -> float: ...
    @friction_velocity.setter
    def friction_velocity(self, value: float) -> None: ...

    @property
    def surface_roughness(self) -> float: ...
    @surface_roughness.setter
    def surface_roughness(self, value: float) -> None: ...

    @property
    def obukhov_length(self) -> float: ...
    @obukhov_length.setter
    def obukhov_length(self, value: float) -> None: ...


class WindEnvironment:
    def __init__(self, setup_string: str) -> None: ...

    def steady_true_wind_velocity_at_location(
        self, *, condition: WindCondition, location: list[float]
    ) -> float: ...

    def steady_true_wind_velocity_vector_at_location(
        self, *, condition: WindCondition, location: list[float]
    ) -> list[float]: ...

    def unsteady_true_wind_velocity_vector_at_location(
        self, *, condition: WindCondition, location: list[float], time: float
    ) -> list[float]: ...

    def steady_apparent_wind_velocity_vector_at_location(
        self,
        *,
        condition: WindCondition,
        location: list[float],
        linear_velocity: list[float]
    ) -> list[float]: ...

    def unsteady_apparent_wind_velocity_vector_at_location(
        self,
        *,
        condition: WindCondition,
        location: list[float],
        linear_velocity: list[float],
        time: float
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
        time: float,
        wing_indices: list[list[int]]
    ) -> list[list[float]]: ...
