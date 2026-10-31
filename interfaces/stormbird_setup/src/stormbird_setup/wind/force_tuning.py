"""
Copyright (C) 2024, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

Tuning of the inflow corrections in the wind environment, so that the forces from a lifting line
simulation match force measurements, for instance from CFD simulations of the sails on a ship.

The functionality runs lifting line simulations, and therefore requires pystormbird. It is an
optional dependency, which can be installed with `pip install stormbird_setup[force-tuning]`. The
rest of stormbird_setup can be used without it.
"""

from typing import Any

from pydantic import Field

from ..base_model import StormbirdSetupBaseModel
from ..spatial_vector import SpatialVector
from ..range import Range
from ..lifting_line.simulation_builder import SimulationBuilder, QuasiSteadySettings

from .wind_environment import WindEnvironment
from .wind_condition import WindCondition
from .inflow_corrections import InflowCorrectionsSingleDirection, InflowCorrections


def _import_pystormbird() -> Any:
    '''
    Imports pystormbird, which is only needed for the force tuning, and therefore an optional
    dependency of stormbird_setup.
    '''
    try:
        import pystormbird.lifting_line # type: ignore[import-not-found]
        import pystormbird.wind # type: ignore[import-not-found]
    except ImportError as error:
        raise ImportError(
            "The force tuning of the inflow corrections requires pystormbird, which is an "
            "optional dependency of stormbird_setup. Install it with "
            "`pip install stormbird_setup[force-tuning]`, or `pip install pystormbird`."
        ) from error

    return pystormbird


class ForceTuningCase(StormbirdSetupBaseModel):
    '''
    A single case to tune the inflow corrections for, i.e., the conditions and the sail settings in
    a force measurement, together with the measured forces on each sail.
    '''
    wind_condition: WindCondition
    """
    The wind condition in the measurement. Must be steady, i.e., without gusts.
    """
    ship_velocity: float = 0.0
    """
    The ship velocity in the measurement. The ship is assumed to sail opposite of the zero direction
    vector in the wind environment, as in the complete sail model.
    """
    local_wing_angles: list[float]
    """
    The local wing angle of each sail in the measurement, in radians, with the same definition as in
    the line force model.
    """
    internal_states: list[float] | None = None
    """
    The internal state of each sail in the measurement, e.g., the revolutions per second of a rotor
    sail. Not set by default, which means that the internal states from the simulation builder are
    used.
    """
    target_forces: list[SpatialVector]
    """
    The measured total force on each sail, in the same coordinate system as the forces from the
    lifting line simulation.
    """


class ForceTuningSettings(StormbirdSetupBaseModel):
    '''
    Settings for the tuning of the inflow corrections
    '''
    magnitude_bounds: tuple[float, float] = (0.8, 1.2)
    """
    The lower and upper bound for the magnitude corrections
    """
    angle_bounds: tuple[float, float] = (-0.35, 0.35)
    """
    The lower and upper bound for the angle corrections, in radians. The default is about +/- 20
    degrees.
    """
    force_weights: SpatialVector = Field(
        default_factory=lambda: SpatialVector(x=1.0, y=1.0, z=1.0)
    )
    """
    Weights for the error in each component of the forces. The default is one for all components,
    i.e., no weighting.
    """
    reference_height: float = 10.0
    """
    The height where the apparent wind direction of each case is computed, which is the direction
    that the corrections belong to. The default is the same as in the complete sail model.
    """
    finite_difference_step: float = 1e-4
    """
    The step used for the finite difference approximation of the derivatives in the optimization.
    Should be larger than the convergence tolerance of the lifting line solver.
    """


class ForceTuningCaseResult(StormbirdSetupBaseModel):
    '''
    The result of the tuning for a single case
    '''
    apparent_wind_direction: float
    """
    The apparent wind direction of the case, computed at the reference height
    """
    magnitude_corrections: list[float]
    """
    The tuned magnitude correction for each sail
    """
    angle_corrections: list[float]
    """
    The tuned angle correction for each sail, in radians
    """
    simulated_forces: list[SpatialVector]
    """
    The total force on each sail from the lifting line simulation with the tuned corrections
    """
    relative_error: float
    """
    The remaining weighted error in the forces, relative to the mean magnitude of the target forces
    """
    success: bool
    """
    Whether the optimization converged
    """
    message: str
    """
    The message from the optimization
    """

    def inflow_corrections(self) -> InflowCorrectionsSingleDirection:
        '''
        Returns the corrections as inflow corrections for a single direction, where the
        corrections are constant along the span of each sail
        '''
        return _constant_corrections(self.magnitude_corrections, self.angle_corrections)


class ForceTuningResult(StormbirdSetupBaseModel):
    '''
    The result of the tuning for multiple cases
    '''
    case_results: list[ForceTuningCaseResult]
    """
    The result for each case, in the same order as the input
    """
    inflow_corrections: InflowCorrections
    """
    The inflow corrections from all cases, sorted by the apparent wind direction
    """


class InflowCorrectionsForceTuner(StormbirdSetupBaseModel):
    '''
    Tunes the inflow corrections in a wind environment, so that the forces from a lifting line
    simulation match measured forces on each sail, for instance from CFD simulations with the full
    geometry of the sails, or with actuator lines.

    For each case, one magnitude correction and one angle correction is found for each sail, which
    are constant along the span. The corrections for all sails are tuned together, as the sails
    affect each other through the induced velocities.

    Requires pystormbird, which is an optional dependency of stormbird_setup.
    '''
    simulation_builder: SimulationBuilder
    """
    Builder for the lifting line simulation. Must use quasi-steady simulation settings.
    """
    wind_environment: WindEnvironment
    """
    The wind environment. Any inflow corrections in it are ignored.
    """
    settings: ForceTuningSettings = Field(default_factory=lambda: ForceTuningSettings())

    def model_post_init(self, context: Any) -> None:
        if not isinstance(self.simulation_builder.simulation_settings, QuasiSteadySettings):
            raise ValueError(
                "The force tuning of the inflow corrections requires quasi-steady simulation settings"
            )

    def apparent_wind_direction(self, case: ForceTuningCase) -> float:
        '''
        Returns the apparent wind direction of a case, computed at the reference height
        '''
        return self.wind_environment.apparent_wind_direction_from_condition_and_linear_velocity(
            case.wind_condition,
            self.wind_environment.linear_velocity_from_ship_velocity(case.ship_velocity),
            self.settings.reference_height
        )

    def sail_forces(
        self,
        case: ForceTuningCase,
        magnitude_corrections: list[float],
        angle_corrections: list[float]
    ) -> list[SpatialVector]:
        '''
        Returns the total force on each sail from the lifting line simulation, for the given case
        and the given corrections, which are constant along the span of each sail.
        '''
        evaluator = _CaseEvaluator(self, case)

        return evaluator.sail_forces(magnitude_corrections, angle_corrections)

    def tune_case(self, case: ForceTuningCase) -> ForceTuningCaseResult:
        '''
        Tunes the corrections for a single case
        '''
        from scipy.optimize import least_squares

        evaluator = _CaseEvaluator(self, case)

        nr_wings = evaluator.nr_wings

        magnitude_bounds = self.settings.magnitude_bounds
        angle_bounds = self.settings.angle_bounds

        # The unknowns are ordered as [magnitude_0, ..., magnitude_n, angle_0, ..., angle_n]
        initial_values = [1.0] * nr_wings + [0.0] * nr_wings

        lower_bounds = [magnitude_bounds[0]] * nr_wings + [angle_bounds[0]] * nr_wings
        upper_bounds = [magnitude_bounds[1]] * nr_wings + [angle_bounds[1]] * nr_wings

        solution = least_squares(
            evaluator.residuals,
            initial_values,
            bounds = (lower_bounds, upper_bounds),
            diff_step = [self.settings.finite_difference_step] * len(initial_values),
            x_scale = 'jac'
        )

        magnitude_corrections = [float(value) for value in solution.x[:nr_wings]]
        angle_corrections = [float(value) for value in solution.x[nr_wings:]]

        residuals = evaluator.residuals(solution.x)

        return ForceTuningCaseResult(
            apparent_wind_direction = self.apparent_wind_direction(case),
            magnitude_corrections = magnitude_corrections,
            angle_corrections = angle_corrections,
            simulated_forces = evaluator.sail_forces(magnitude_corrections, angle_corrections),
            relative_error = float(sum(value**2 for value in residuals)**0.5),
            success = bool(solution.success),
            message = str(solution.message)
        )

    def tune(self, cases: list[ForceTuningCase]) -> ForceTuningResult:
        '''
        Tunes the corrections for all cases, and combines them into inflow corrections, sorted by
        the apparent wind direction. The cases must have different apparent wind directions.
        '''
        if len(cases) == 0:
            raise ValueError("At least one case must be given")

        case_results = [self.tune_case(case) for case in cases]

        sorted_results = sorted(case_results, key=lambda result: result.apparent_wind_direction)

        for previous, current in zip(sorted_results[:-1], sorted_results[1:]):
            if abs(current.apparent_wind_direction - previous.apparent_wind_direction) < 1e-6:
                raise ValueError(
                    "Two of the cases have the same apparent wind direction "
                    f"({current.apparent_wind_direction} rad). The inflow corrections can only "
                    "have one set of corrections for each direction, so the cases should be tuned "
                    "separately, e.g., with tune_case."
                )

        inflow_corrections = InflowCorrections(
            apparent_wind_directions = [
                result.apparent_wind_direction for result in sorted_results
            ],
            corrections = [result.inflow_corrections() for result in sorted_results]
        )

        return ForceTuningResult(
            case_results = case_results,
            inflow_corrections = inflow_corrections
        )


def _constant_corrections(
    magnitude_corrections: list[float],
    angle_corrections: list[float]
) -> InflowCorrectionsSingleDirection:
    '''
    Inflow corrections for a single direction, with one value for each sail, which is then constant
    along the span
    '''
    nr_wings = len(magnitude_corrections)

    return InflowCorrectionsSingleDirection(
        height_values = [0.0] * nr_wings,
        magnitude_corrections = list(magnitude_corrections),
        angle_corrections = list(angle_corrections),
        wing_indices = [Range(start = i, end = i + 1) for i in range(nr_wings)]
    )


class _CaseEvaluator:
    '''
    Computes the forces from the lifting line simulation for a single case, with different
    corrections. The simulation is built once, and reused for all evaluations.
    '''
    def __init__(self, tuner: InflowCorrectionsForceTuner, case: ForceTuningCase):
        pystormbird = _import_pystormbird()

        if (
            case.wind_condition.parallel_gust is not None or
            case.wind_condition.perpendicular_gust is not None or
            case.wind_condition.vertical_gust is not None
        ):
            raise ValueError("The wind condition in the force tuning must be steady, without gusts")

        self.pystormbird = pystormbird
        self.tuner = tuner
        self.case = case

        self.simulation = pystormbird.lifting_line.Simulation(
            tuner.simulation_builder.to_json_string()
        )

        self.wing_indices = self.simulation.line_force_model.wing_indices
        self.nr_wings = len(self.wing_indices)

        values_to_check: list[tuple[str, list[Any] | None]] = [
            ("local_wing_angles", case.local_wing_angles),
            ("target_forces", case.target_forces),
            ("internal_states", case.internal_states),
        ]

        for name, values in values_to_check:
            if values is not None and len(values) != self.nr_wings:
                raise ValueError(
                    f"The number of values in {name} ({len(values)}) is different from the number "
                    f"of wings in the simulation ({self.nr_wings})"
                )

        self.simulation.set_local_wing_angles(case.local_wing_angles)

        if case.internal_states is not None:
            self.simulation.set_internal_state(case.internal_states)

        # Only the control points, as the simulation is quasi-steady
        self.ctrl_points = self.simulation.get_freestream_velocity_points()

        self.wind_condition = pystormbird.wind.WindCondition.from_json_string(
            case.wind_condition.to_json_string()
        )

        self.linear_velocity = tuner.wind_environment.linear_velocity_from_ship_velocity(
            case.ship_velocity
        ).as_list()

        self.reference_force = sum(
            force.length() for force in case.target_forces
        ) / self.nr_wings

        if self.reference_force == 0.0:
            raise ValueError("The target forces can not all be zero")

    def sail_forces(
        self,
        magnitude_corrections: list[float],
        angle_corrections: list[float]
    ) -> list[SpatialVector]:
        # The corrections are given for a single direction, so they are used for all directions
        inflow_corrections = InflowCorrections(
            apparent_wind_directions = [0.0],
            corrections = [_constant_corrections(magnitude_corrections, angle_corrections)]
        )

        wind_environment = self.tuner.wind_environment.model_copy(
            update = {"inflow_corrections": inflow_corrections}
        )

        wind_environment_rust = self.pystormbird.wind.WindEnvironment(
            wind_environment.to_json_string()
        )

        freestream_velocity = wind_environment_rust.apparent_wind_velocity_vectors_at_ctrl_points_with_corrections_applied(
            condition = self.wind_condition,
            ctrl_points = self.ctrl_points,
            linear_velocity = self.linear_velocity,
            time = 0.0,
            wing_indices = self.wing_indices
        )

        # Makes the result independent of the previous evaluations, which is needed for the finite
        # difference approximations in the optimization
        self.simulation.reset_previous_circulation_strength()

        result = self.simulation.do_step(
            time = 0.0,
            time_step = 1.0,
            freestream_velocity = freestream_velocity
        )

        return [
            SpatialVector.from_list(list(values.total)) for values in result.integrated_forces
        ]

    def residuals(self, values: Any) -> list[float]:
        magnitude_corrections = [float(value) for value in values[:self.nr_wings]]
        angle_corrections = [float(value) for value in values[self.nr_wings:]]

        forces = self.sail_forces(magnitude_corrections, angle_corrections)

        weights = self.tuner.settings.force_weights

        out = []

        for force, target in zip(forces, self.case.target_forces):
            error = force - target

            out.append(weights.x * error.x / self.reference_force)
            out.append(weights.y * error.y / self.reference_force)
            out.append(weights.z * error.z / self.reference_force)

        return out
