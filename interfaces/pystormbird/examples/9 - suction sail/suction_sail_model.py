'''
Functions for setting up and running simulations of the three-dimensional suction sail model from
the paper, shared by the scripts in this example.
'''

from dataclasses import dataclass
from pathlib import Path

import numpy as np

from stormbird_setup.section_models import (
    SectionModel,
    ActiveAirFlowFoil,
    VaryingFoil,
    InternalStateConversion,
    InternalStateConversionType,
    InternalFlow,
)
from stormbird_setup.spatial_vector import SpatialVector
from stormbird_setup.line_force_model import LineForceModelBuilder, WingBuilder
from stormbird_setup.circulation_corrections import CirculationCorrectionBuilder
from stormbird_setup.lifting_line.simulation_builder import SimulationBuilder, QuasiSteadySettings
from stormbird_setup.lifting_line.solver import Iterative
from stormbird_setup.lifting_line.wake import QuasiSteadyWakeSettings, SymmetryCondition

from pystormbird.lifting_line import Simulation

from plot_sectional_data import VARYING_FOIL_FILE
from suction_sail_data import ASPECT_RATIO_3D

CHORD_LENGTH = 1.0
HEIGHT = ASPECT_RATIO_3D * CHORD_LENGTH
AREA = CHORD_LENGTH * HEIGHT

NR_SECTIONS = 20

# Virtual extension at the top of the wing, as a fraction of the span. This increases the induced
# drag to better match the three-dimensional data.
VIRTUAL_EXTENSION_FACTOR = 0.15

# Gaussian smoothing of the circulation, given as a fraction of the span. The smoothing stabilizes
# the solution close to and after stall, where the lift can otherwise jump between different
# solutions from one angle of attack to the next.
SMOOTHING_LENGTH_FACTOR = 0.1

VELOCITY = 10.0
DENSITY = 1.225


# The varying foil model fitted to the two-dimensional data, and the same model retuned to the
# three-dimensional data
RAW_VARYING_FOIL_FILE = VARYING_FOIL_FILE
RETUNED_VARYING_FOIL_FILE = Path(__file__).parent / "retuned_varying_foil.json"

# TODO: The retuning to the three-dimensional data in retune_section_model.py must be updated to the
# new slope reduction in the foil model. Until then, the model fitted to the two-dimensional data is
# used in all steps. Set this to True when the retuning is updated.
USE_RETUNED_SECTION_MODEL = False


def load_varying_foil(file_path: Path, script_name: str) -> VaryingFoil:
    if not file_path.exists():
        raise FileNotFoundError(f"{file_path.name} does not exist. Run {script_name} first.")

    return VaryingFoil.from_json_file(file_path)


def load_raw_varying_foil() -> VaryingFoil:
    '''
    The varying foil model fitted to the two-dimensional data in the first step
    '''
    return load_varying_foil(RAW_VARYING_FOIL_FILE, "plot_sectional_data.py")


def load_retuned_varying_foil() -> VaryingFoil:
    '''
    The varying foil model retuned to the three-dimensional data in the second step
    '''
    return load_varying_foil(RETUNED_VARYING_FOIL_FILE, "retune_section_model.py")


def load_section_varying_foil() -> VaryingFoil:
    '''
    The varying foil model used when tuning the power model: the retuned model if
    `USE_RETUNED_SECTION_MODEL` is set, and the model fitted to the two-dimensional data otherwise
    '''
    if USE_RETUNED_SECTION_MODEL:
        return load_retuned_varying_foil()

    return load_raw_varying_foil()


def section_model(
    internal_state_conversion: InternalStateConversion,
    varying_foil: VaryingFoil
) -> SectionModel:
    return SectionModel(
        model = ActiveAirFlowFoil(
            varying_foil = varying_foil,
            internal_state_conversion = internal_state_conversion
        )
    )


def suction_coefficient_direct() -> InternalStateConversion:
    return InternalStateConversion(
        conversion_type = InternalStateConversionType.SuctionCoefficientDirect
    )


def power_to_internal_flow(internal_flow: InternalFlow, fan_efficiency: float) -> InternalStateConversion:
    return InternalStateConversion.new_power_to_internal_flow(internal_flow, fan_efficiency)


def first_maximum(values: np.ndarray) -> int:
    '''
    The index of the first local maximum, which is used to find the maximum lift before stall. The
    lift can increase again far after stall, so the global maximum is not used.
    '''
    for i in range(len(values) - 1):
        if values[i + 1] < values[i]:
            return i

    return len(values) - 1


def create_simulation(
    internal_state_conversion: InternalStateConversion, 
    varying_foil: VaryingFoil
) -> Simulation:
    '''
    Creates a steady simulation of the three-dimensional model in the paper, which had one end
    standing directly on the wall of the wind tunnel. This is modeled with a symmetry plane at
    z = 0, where the bottom of the wing is placed, which gives an effective aspect ratio that is
    twice the geometric aspect ratio.
    
    A virtual extension is added at the top of the wing to increase the induced drag.
    '''
    chord_vector = SpatialVector(x=CHORD_LENGTH)

    virtual_extension_length = VIRTUAL_EXTENSION_FACTOR * HEIGHT

    # The wing is oriented along the z-axis, starting at the symmetry plane. The circulation is not
    # zero at the bottom of the wing, as the wing continues as a mirror image below the plane.
    # A virtual extension is added at the top to increase induced drag.
    wing_builder = WingBuilder(
        section_points = [
            SpatialVector(x=0.0, y=0.0, z=0.0),
            SpatialVector(x=0.0, y=0.0, z=HEIGHT),
            SpatialVector(x=0.0, y=0.0, z=HEIGHT + virtual_extension_length),
        ],
        chord_vectors = [chord_vector, chord_vector, chord_vector],
        section_model = section_model(internal_state_conversion, varying_foil),
        non_zero_circulation_at_ends = (True, False),
        line_segment_is_virtual = [False, True],
    )

    line_force_model = LineForceModelBuilder(
        nr_sections = NR_SECTIONS,
        density = DENSITY,
        circulation_correction = CirculationCorrectionBuilder.new_gaussian_smoothing(
            SMOOTHING_LENGTH_FACTOR
        )
    )

    line_force_model.add_wing_builder(wing_builder)

    # The iterative solver is used, as it handles the behavior close to stall better than the
    # linearized solver
    simulation_settings = QuasiSteadySettings(
        solver = Iterative(
            max_iterations_per_time_step = 1000,
            damping_factor = 0.05,
            start_with_linearized_solution = True
        ),
        wake = QuasiSteadyWakeSettings(
            symmetry_condition = SymmetryCondition.Z
        )
    )

    simulation_builder = SimulationBuilder(
        line_force_model = line_force_model,
        simulation_settings = simulation_settings
    )

    return Simulation(simulation_builder.to_json_string())


@dataclass
class SimulationResult:
    cl: float
    cd: float
    cp: float
    section_internal_states: np.ndarray
    section_angles_of_attack: np.ndarray
    section_velocities: np.ndarray

    def wing_suction_coefficient(self) -> float:
        '''
        The suction coefficient for the whole wing, Q / (U A), from the suction coefficient on each
        line element, which all have the same area.
        '''
        return np.mean(np.abs(self.section_internal_states) * self.section_velocities) / VELOCITY


def run_case(simulation: Simulation, angle_of_attack_deg: float, internal_state: float) -> SimulationResult:
    '''
    Runs a steady simulation for one angle of attack and internal state, and returns the force and
    power coefficients for the wing.
    '''
    freestream_velocity = [
        [VELOCITY, 0.0, 0.0] for _ in simulation.get_freestream_velocity_points()
    ]

    simulation.set_local_wing_angles([-np.radians(angle_of_attack_deg)])
    simulation.set_internal_state([internal_state])

    result = simulation.do_step(
        time = 0.0,
        time_step = 1.0,
        freestream_velocity = freestream_velocity
    )

    force_factor = 0.5 * DENSITY * AREA * VELOCITY**2

    force = result.integrated_forces[0].total

    return SimulationResult(
        cl = force[1] / force_factor,
        cd = force[0] / force_factor,
        cp = result.input_power[0] / (force_factor * VELOCITY),
        section_internal_states = np.array(result.force_input.section_models_internal_state),
        section_angles_of_attack = np.array(result.force_input.angles_of_attack),
        section_velocities = np.linalg.norm(np.array(result.force_input.velocity), axis=1),
    )


def angle_sweep(
    simulation: Simulation,
    angles_of_attack_deg: np.ndarray,
    internal_state: float
) -> list[SimulationResult]:
    return [run_case(simulation, aoa, internal_state) for aoa in angles_of_attack_deg]
