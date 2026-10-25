'''
Shared setup for the single rotor sail examples, used by both the Stormflow example in this folder
and the OpenFOAM example in interfaces/cfd_interfaces/openfoam/examples/single_rotor_spin_ratios.
The actuator line model is only defined here, so that both solvers use exactly the same settings.

The rotor has the same dimensions as in the lifting line example in
interfaces/pystormbird/examples/3 - rotor_sails, and the results are compared against the same data
from Deybach et al. (2024).
'''

import json
from enum import Enum
from pathlib import Path

import numpy as np
from stormbird_setup.actuator_line import (
    ActuatorLineBuilder,
    EmpiricalCirculationCorrection,
    Gaussian,
    LiftingLineCorrectionBuilder,
    ProjectionSettings,
    SamplingSettings,
    SolverSettings,
    SymmetryCondition,
)
from stormbird_setup.circulation_corrections import CirculationCorrectionBuilder
from stormbird_setup.controller import (
    ControllerBuilder,
    ControllerSetPoints,
    InternalStateSetPointType,
)
from stormbird_setup.input_power import InputPowerDataType, InputPowerModel
from stormbird_setup.line_force_model import LineForceModelBuilder, WingBuilder, ForceCorrection
from stormbird_setup.section_models import RotatingCylinder, SectionModel
from stormbird_setup.spatial_vector import SpatialVector
from stormbird_setup.stormflow.geometry import GeometryBuilder
from stormbird_setup.utils import revolutions_per_second_from_spin_ratio
from stormbird_setup.wind import WindCondition

DIAMETER = 5.0
HEIGHT = 30.0
FOUNDATION_HEIGHT = 1.875
VELOCITY = 8.0

# The foundation is a box with the same width as the rotor diameter, from the ground up to the rotor.
# It extends below the ground, so that none of its faces are on the boundary of the domain.
FOUNDATION_WIDTH = DIAMETER
FOUNDATION_DEPTH_BELOW_GROUND = 1.0

# End disk at the top of the rotor. The thickness is two cells at 16 cells per diameter.
END_DISK_DIAMETER = 1.5 * DIAMETER
END_DISK_THICKNESS = 0.625

# Linear lift slope, manually tuned so that the lift match the experimental data at spin ratio of 3
LIFT_SLOPE = 2.74

# Limit on the velocity correction from the lifting line correction, as a ratio of the sampled
# velocity. Stabilizes the simulations, and avoids a too large correction at the ends of the rotor.
MAX_CORRECTION_RATIO = 1.0

# Both solvers are incompressible, with the pressure divided by the density. The projected forces
# are not divided by the density, so it must be one in the line force model.
DENSITY = 1.0

NR_SECTIONS = 32

# Grid resolution at the rotor, used by both solvers. The chord length is the diameter.
CELLS_PER_CHORD_LENGTH = 16

# Default settings for the mesh convergence tests. The resolutions are run from the lowest to the
# highest.
MESH_TEST_SPIN_RATIO = 3.0
MESH_TEST_CELLS_PER_CHORD_LENGTH = [4, 8, 16, 32]

SPIN_RATIOS = np.arange(0.5, 4.01, 0.5).tolist()

# The rotor speed is increased linearly from zero over this time, to get a smooth start
SPIN_UP_TIME = 5.0

END_TIME = 20.0

# The forces are averaged over the last part of the simulation
AVERAGING_TIME = 5.0

class Variant(Enum):
    '''
    The actuator line model is tested both with and without the lifting line correction
    '''
    NoCorrections = "no_corrections"
    LiftingLineCorrection = "ll_correction"
    EmpiricalCirculationCorrection = "empirical_correction"
    

    @property
    def label(self) -> str:
        match self:
            case Variant.LiftingLineCorrection:
                return "lifting line correction"
            case Variant.NoCorrections:
                return "no corrections"
            case Variant.EmpiricalCirculationCorrection:
                return "empirical correction"

def force_factor() -> float:
    return 0.5 * DENSITY * DIAMETER * HEIGHT * VELOCITY**2

def wind_condition() -> WindCondition:
    '''
    Constant wind along the positive x-axis
    '''
    return WindCondition(direction_coming_from=0.0, velocity_variation=VELOCITY)

def section_model() -> SectionModel:
    '''
    Rotating cylinder with a linear lift. The drag uses the default values in Stormbird.
    '''
    return SectionModel(
        model=RotatingCylinder(cl_initial_slope=LIFT_SLOPE, cl_high_spin_ratio_slope=LIFT_SLOPE)
    )

def slip_geometries() -> list[GeometryBuilder]:
    '''
    The foundation and the end disk, as in the experiments. Both are modelled as slip surfaces.
    '''
    foundation_bottom = -FOUNDATION_DEPTH_BELOW_GROUND
    foundation_half_height = 0.5 * (FOUNDATION_HEIGHT - foundation_bottom)

    return [
        GeometryBuilder.new_cuboid(
            center=SpatialVector(z=foundation_bottom + foundation_half_height),
            half_extents=SpatialVector(
                x=0.5 * FOUNDATION_WIDTH, y=0.5 * FOUNDATION_WIDTH, z=foundation_half_height
            ),
        ),
        GeometryBuilder.new_disk(
            center=SpatialVector(z=FOUNDATION_HEIGHT + HEIGHT),
            normal=SpatialVector(z=1.0),
            radius=0.5 * END_DISK_DIAMETER,
            thickness=END_DISK_THICKNESS,
        ),
    ]

def actuator_line_builder(spin_ratio: float, variant: Variant) -> ActuatorLineBuilder:
    line_force_model = LineForceModelBuilder(
        nr_sections=NR_SECTIONS,
        density=DENSITY,
        circulation_correction=CirculationCorrectionBuilder.new_gaussian_smoothing(
            smoothing_length_factor=0.05
        ),
    )

    line_force_model.add_wing_builder(
        WingBuilder(
            section_points=[
                SpatialVector(z=FOUNDATION_HEIGHT),
                SpatialVector(z=FOUNDATION_HEIGHT + HEIGHT)
            ],
            chord_vectors=[SpatialVector(x=DIAMETER), SpatialVector(x=DIAMETER)],
            section_model=section_model(),
            non_zero_circulation_at_ends=(True, True),
            input_power_model=InputPowerModel(input_power_type=InputPowerDataType.NoPower)
        )
    )

    # The internal state can not be given directly in the setup, so a controller with a constant set
    # point is used. The set point is given directly as revolutions per second, computed from the
    # freestream velocity, so that the spin ratio is defined in the same way as in the comparison
    # data.
    revolutions_per_second = revolutions_per_second_from_spin_ratio(
        spin_ratio=spin_ratio,
        diameter=DIAMETER,
        velocity=VELOCITY
    )

    controller = ControllerBuilder(
        set_points=[
            ControllerSetPoints(
                apparent_wind_directions_data=[-np.pi, np.pi],
                internal_state_data=[revolutions_per_second, revolutions_per_second],
                internal_state_set_point_type=InternalStateSetPointType.Wing,
                wing_internal_state_max_change_rate=(
                    max(abs(revolutions_per_second), 1e-6) / SPIN_UP_TIME
                ),
            )
        ]
    )

    match variant:
        case Variant.EmpiricalCirculationCorrection:
            empirical_circulation_correction = EmpiricalCirculationCorrection(
                exp_factor = 16.0
            )
            lifting_line_correction = LiftingLineCorrectionBuilder(
                symmetry_condition=SymmetryCondition.Z,
                max_correction_ratio=MAX_CORRECTION_RATIO
            )
        case Variant.LiftingLineCorrection:
            lifting_line_correction = LiftingLineCorrectionBuilder(
                symmetry_condition=SymmetryCondition.Z,
                max_correction_ratio=MAX_CORRECTION_RATIO
            )
            empirical_circulation_correction = None
        case Variant.NoCorrections:
            lifting_line_correction = None
            empirical_circulation_correction = None

    return ActuatorLineBuilder(
        line_force_model=line_force_model,
        projection_settings=ProjectionSettings(
            projection_function=Gaussian(chord_factor=0.6, thickness_factor=0.6)
        ),
        solver_settings=SolverSettings(damping_factor=0.05),
        sampling_settings=SamplingSettings(remove_span_velocity=True),
        empirical_circulation_correction=empirical_circulation_correction,
        lifting_line_correction=lifting_line_correction,
        controller=controller,
        write_iterations_full_result=100,
    )

def force_coefficients(
    time: list[float],
    force_x: list[float],
    force_y: list[float]
) -> dict[str, float]:
    '''
    Drag and lift coefficients, averaged over the last part of the simulation. The wind is along
    the x-axis, so the drag is the force along x and the lift the force along y.
    '''
    time_array = np.array(time)

    in_averaging_window = time_array >= time_array[-1] - AVERAGING_TIME

    return {
        "cd": float(np.mean(np.array(force_x)[in_averaging_window]) / force_factor()),
        "cl": float(np.mean(np.array(force_y)[in_averaging_window]) / force_factor()),
    }

def write_results(results: dict, file_path: Path) -> None:
    file_path.write_text(json.dumps(results, indent=4))

def read_results(file_path: Path) -> dict:
    if not file_path.exists():
        return {}

    return json.loads(file_path.read_text())
