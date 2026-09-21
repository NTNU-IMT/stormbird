"""Type stubs for pystormbird.lifting_line."""

from pystormbird import SimulationResult, SingleSailResult
from .line_force_model import LineForceModel
from .wind import WindCondition, WindEnvironment


class Simulation:
    def __init__(self, setup_string: str) -> None: ...

    @property
    def line_force_model(self) -> LineForceModel: ...

    def set_translation_and_rotation_with_finite_difference_for_the_velocity(
        self, time_step: float, translation: list[float], rotation: list[float]
    ) -> None: ...

    def set_translation_with_velocity_using_finite_difference(
        self, translation: list[float], time_step: float
    ) -> None: ...

    def set_rotation_with_velocity_using_finite_difference(
        self, rotation: list[float], time_step: float
    ) -> None: ...

    def set_translation_only(self, translation: list[float]) -> None: ...
    def set_rotation_only(self, rotation: list[float]) -> None: ...
    def set_velocity_linear(self, linear_velocity: list[float]) -> None: ...
    def set_velocity_angular(self, angular_velocity: list[float]) -> None: ...
    def reset_previous_circulation_strength(self) -> None: ...
    def set_local_wing_angles(self, local_wing_angles: list[float]) -> None: ...
    def set_section_models_internal_state(self, internal_state: list[float]) -> None: ...
    def get_freestream_velocity_points(self) -> list[list[float]]: ...

    def do_step(
        self,
        *,
        time: float,
        time_step: float,
        freestream_velocity: list[list[float]]
    ) -> SimulationResult: ...

    def induced_velocities(self, points: list[list[float]]) -> list[list[float]]: ...
    def dynamic_wake_as_vtk(self, binary: bool = True) -> bytes | None: ...


class CompleteSailModel:
    def __init__(self, setup_string: str) -> None: ...

    def get_number_of_sails(self) -> int:
        """The number of sails in the model."""
        ...

    def simulate_condition_optimal_controller_loading(
        self,
        *,
        wind_condition: WindCondition,
        ship_velocity: float,
        nr_loadings_to_test: int
    ) -> SimulationResult:
        """
        Runs multiple steady state simulations with different controller loadings, and returns the
        result with the highest effective power. That is, the power delivered by the thrust, minus
        the power that is used by the sails.
        """
        ...

    def simulate_steady_state_condition(
        self,
        *,
        wind_condition: WindCondition,
        ship_velocity: float,
        controller_loading: float
    ) -> SimulationResult:
        """Applies the controller and simulates a single steady state condition."""
        ...

    def simulate_steady_state_condition_simple_output(
        self,
        *,
        wind_condition: WindCondition,
        ship_velocity: float,
        controller_loading: float
    ) -> list[SingleSailResult]:
        """
        Same as `simulate_steady_state_condition`, but the result is simplified to the total force
        and moment and the input power for each sail.
        """
        ...

    def apply_controller_based_on_wind_condition(
        self,
        *,
        time: float,
        time_step: float,
        wind_condition: WindCondition,
        ship_velocity: float,
        controller_loading: float
    ) -> None: ...

    def freestream_velocity(
        self,
        *,
        wind_condition: WindCondition,
        ship_velocity: float,
        time: float
    ) -> list[list[float]]:
        """
        The freestream velocity at all the points where it is needed by the simulation. That is,
        the apparent wind, including the effect of the ship velocity and inflow corrections.
        """
        ...

    def apply_controller_based_on_freestream(
        self,
        *,
        time: float,
        time_step: float,
        loading: float,
        freestream_velocity: list[list[float]]
    ) -> None: ...

    def apply_controller_based_on_simulation_result(
        self,
        *,
        time: float,
        time_step: float,
        loading: float,
        simulation_result: SimulationResult
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
