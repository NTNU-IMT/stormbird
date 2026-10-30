'''
Shared setup for the rotor sail examples, used by both the Stormflow examples in this folder and the
OpenFOAM examples in interfaces/cfd_interfaces/openfoam/examples/rotor_sails/deybach_wind_tunnel.
The actuator line model and the flow settings are only defined here, so that both solvers use
exactly the same settings. The model comes in several variants, which differ in how the tip vortices
are stabilized and how the lift-induced drag is corrected. See the README for a description of each
variant.

The rotor and the wind tunnel have the same dimensions as in the experiments by Deybach et al.
(2024, 2026), scaled to full scale. The rotor has the same dimensions as in the lifting line
examples in interfaces/pystormbird/examples/rotor_sails/deybach_wind_tunnel, and the results are
compared against the same data.
'''

import argparse
from enum import Enum

import numpy as np
from stormbird_setup.actuator_line import (
    ActuatorLineBuilder,
    Gaussian,
    LiftingLineCorrectionBuilder,
    ProjectionSettings,
    SamplingSettings,
    SolverSettings,
    SymmetryCondition,
    TipLossApplication,
    TipLossCorrection,
)
from stormbird_setup.circulation_corrections import CirculationCorrectionBuilder
from stormbird_setup.controller import (
    ControllerBuilder,
    ControllerSetPoints,
    InternalStateSetPointType,
)
from stormbird_setup.input_power import InputPowerDataType, InputPowerModel
from stormbird_setup.line_force_model import (
    ForceCorrection,
    LineForceModelBuilder,
    WingBuilder,
)
from stormbird_setup.section_models import RotatingCylinder, SectionModel
from stormbird_setup.spatial_vector import SpatialVector
from stormbird_setup.utils import revolutions_per_second_from_spin_ratio
from stormbird_setup.wind import WindCondition

# The experiments are done in model scale, but the simulations are run in full scale. All dimensions
# of the rotor and the wind tunnel are scaled with the same factor.
MODEL_SCALE_DIAMETER = 0.4
MODEL_SCALE_HEIGHT = 2.4
MODEL_SCALE_FOUNDATION_HEIGHT = 0.15

MODEL_SCALE_TUNNEL_LENGTH = 15.0
MODEL_SCALE_TUNNEL_WIDTH = 5.0
MODEL_SCALE_TUNNEL_HEIGHT = 3.0

DIAMETER = 5.0
SCALE_FACTOR = DIAMETER / MODEL_SCALE_DIAMETER

HEIGHT = MODEL_SCALE_HEIGHT * SCALE_FACTOR
ASPECT_RATIO = HEIGHT/DIAMETER
FOUNDATION_HEIGHT = MODEL_SCALE_FOUNDATION_HEIGHT * SCALE_FACTOR
VELOCITY = 8.0

# The simulation domain is the wind tunnel. The rotor at the origin is centered in the width of the
# tunnel, with the inlet this distance upstream.
TUNNEL_LENGTH = MODEL_SCALE_TUNNEL_LENGTH * SCALE_FACTOR
TUNNEL_WIDTH = MODEL_SCALE_TUNNEL_WIDTH * SCALE_FACTOR
TUNNEL_HEIGHT = MODEL_SCALE_TUNNEL_HEIGHT * SCALE_FACTOR
TUNNEL_INLET_DISTANCE = 75.0

# Linear lift slope, manually tuned so that the lift match the experimental data at a spin ratio
# around 1
LIFT_SLOPE = 2.6

# Limit on the velocity correction from the lifting line correction, as a ratio of the sampled
# velocity. Stabilizes the simulations, and avoids a too large correction at the ends of the rotor.
MAX_CORRECTION_RATIO = 2.25

# Both solvers are incompressible, with the pressure divided by the density. The projected forces
# are not divided by the density, so it must be one in the line force model.
DENSITY = 1.0

NR_SECTIONS = 40

# Both solvers are run without a turbulence model, with this constant kinematic viscosity in the
# whole domain
EFFECTIVE_VISCOSITY = 1.5e-5

# Maximum Courant number, used to set the time step in both solvers
COURANT_NUMBER = 0.75

# Grid resolution at the rotor, used by both solvers. The chord length is the diameter.
CELLS_PER_CHORD_LENGTH = 10

# Default settings for the mesh convergence tests. The resolutions are run from the lowest to the
# highest.
MESH_TEST_SPIN_RATIO = 3.0
MESH_TEST_CELLS_PER_CHORD_LENGTH = [4, 6, 8, 10]

SPIN_RATIOS = np.arange(0.5, 4.01, 0.5).tolist()

# The rotor speed is increased linearly from zero over this time, to get a smooth start
SPIN_UP_TIME = 5.0

END_TIME = 40.0

# The forces are averaged over the last part of the simulation
AVERAGING_TIME = 10.0

# Settings for the two rotor case, the same as in the lifting line example and the experiments by
# Deybach et al. (2026). The interfering rotor is placed at different angular positions around the
# rotor at the origin.
TWO_ROTORS_SPIN_RATIO = 2.5
TWO_ROTORS_SPACING = 4.0 * DIAMETER

class Variant(Enum):
    '''
    Variants of the actuator line model, with different combinations of the lifting line
    correction, the tip loss correction and additional lift-induced drag
    '''
    NoCorrections = "no_corrections"
    LiftingLine = "lifting_line"
    TipLossAndLiftingLine = "tip_loss_and_lifting_line"
    TipLossAndLiftingLineAndAdditionalCDi = "tip_loss_and_lifting_line_and_additional_cdi"
    ProjectedTipLossAndLiftingLine = "projected_tip_loss_and_lifting_line"

    @property
    def label(self) -> str:
        match self:
            case Variant.ProjectedTipLossAndLiftingLine:
                return "projected tip loss and lifting line"
            case Variant.LiftingLine:
                return "lifting line"
            case Variant.NoCorrections:
                return "no corrections"
            case Variant.TipLossAndLiftingLine:
                return "tip loss and lifting line"
            case Variant.TipLossAndLiftingLineAndAdditionalCDi:
                return "tip loss and lifting line and additional CDi"

# The recommended variant, used when no variant is given to the scripts
DEFAULT_VARIANT = Variant.ProjectedTipLossAndLiftingLine

TIP_LOSS_EXP_FACTOR = 16

# Shorthand for all variants on the command line
ALL_VARIANTS = "all"
VARIANT_CHOICES = [variant.value for variant in Variant] + [ALL_VARIANTS]

def variants_from_arguments(values: list[str]) -> list[Variant]:
    '''
    Converts the variant values given on the command line to variants. The value ALL_VARIANTS
    selects all variants.
    '''
    if ALL_VARIANTS in values:
        return list(Variant)

    return [Variant(value) for value in values]

def add_variant_argument(parser: argparse.ArgumentParser, action: str) -> None:
    '''
    Adds the --variant argument to a command line parser, where one or more variants can be given,
    or ALL_VARIANTS. The action is the verb used in the help text, e.g., "simulate".
    '''
    parser.add_argument(
        "--variant", nargs="+", default=[DEFAULT_VARIANT.value], choices=VARIANT_CHOICES,
        help=f"One or more variants to {action}, or '{ALL_VARIANTS}' to {action} all"
    )

def interfering_rotor_location(angular_position_deg: float) -> SpatialVector:
    '''
    Location of the interfering rotor in the two rotor case. An angular position of 0 degrees means
    that the interfering rotor is directly upstream of the rotor at the origin. The definition is
    the same as in the lifting line example.
    '''
    return SpatialVector(x=-TWO_ROTORS_SPACING).rotate_around_axis(
        np.radians(angular_position_deg),
        axis=SpatialVector(z=1.0)
    )

def two_rotors_locations(angular_position_deg: float) -> list[SpatialVector]:
    '''
    The locations of both rotors in the two rotor case. The rotor at the origin is first, so that
    the forces on it are the forces on the first wing in the results.
    '''
    return [SpatialVector(), interfering_rotor_location(angular_position_deg)]

def tunnel_start_point() -> SpatialVector:
    '''
    The corner of the wind tunnel at the inlet, on the ground. Used as the start of the domain.
    '''
    return SpatialVector(x=-TUNNEL_INLET_DISTANCE, y=-0.5 * TUNNEL_WIDTH, z=0.0)

def tunnel_end_point() -> SpatialVector:
    '''
    The corner of the wind tunnel at the outlet, at the top. Used as the end of the domain.
    '''
    return SpatialVector(
        x=TUNNEL_LENGTH - TUNNEL_INLET_DISTANCE, y=0.5 * TUNNEL_WIDTH, z=TUNNEL_HEIGHT
    )

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

def actuator_line_builder(
    spin_ratio: float,
    variant: Variant,
    rotor_locations: list[SpatialVector] | None = None
) -> ActuatorLineBuilder:
    '''
    Actuator line model of one or more rotors, all with the same spin ratio. The locations are the
    positions of the rotors on the ground. A single rotor at the origin is used by default.
    '''
    if rotor_locations is None:
        rotor_locations = [SpatialVector()]

    line_force_model = LineForceModelBuilder(
        nr_sections=NR_SECTIONS,
        density=DENSITY,
        circulation_correction=CirculationCorrectionBuilder.new_gaussian_smoothing(
            smoothing_length_factor=0.05
        ),
    )

    match variant:
        case Variant.TipLossAndLiftingLineAndAdditionalCDi:
            lift_induced_drag_correction = 0.35/(np.pi*ASPECT_RATIO)
        case _:
            lift_induced_drag_correction = 0.0


    for location in rotor_locations:
        line_force_model.add_wing_builder(
            WingBuilder(
                section_points=[
                    SpatialVector(x=location.x, y=location.y, z=FOUNDATION_HEIGHT),
                    SpatialVector(x=location.x, y=location.y, z=FOUNDATION_HEIGHT + HEIGHT)
                ],
                chord_vectors=[SpatialVector(x=DIAMETER), SpatialVector(x=DIAMETER)],
                section_model=section_model(),
                non_zero_circulation_at_ends=(True, True),
                input_power_model=InputPowerModel(input_power_type=InputPowerDataType.NoPower),
                force_correction = ForceCorrection(
                    lift_induced_drag_correction = lift_induced_drag_correction
                )
            )
        )

    # The internal state can not be given directly in the setup, so a controller with a constant set
    # point is used. The set point is given directly as revolutions per second, computed from the
    # freestream velocity, so that the spin ratio is defined in the same way as in the comparison
    # data. The controller needs one set point for each rotor.
    revolutions_per_second = revolutions_per_second_from_spin_ratio(
        spin_ratio=spin_ratio,
        diameter=DIAMETER,
        velocity=VELOCITY
    )

    set_points = ControllerSetPoints(
        apparent_wind_directions_data=[-np.pi, np.pi],
        internal_state_data=[revolutions_per_second, revolutions_per_second],
        internal_state_set_point_type=InternalStateSetPointType.Wing,
        wing_internal_state_max_change_rate=(
            max(abs(revolutions_per_second), 1e-6) / SPIN_UP_TIME
        ),
    )

    controller = ControllerBuilder(set_points=[set_points] * len(rotor_locations))

    match variant:
        case Variant.TipLossAndLiftingLine | Variant.TipLossAndLiftingLineAndAdditionalCDi:
            tip_loss_correction = TipLossCorrection(
                exp_factor = TIP_LOSS_EXP_FACTOR
            )
            lifting_line_correction = LiftingLineCorrectionBuilder(
                symmetry_condition=SymmetryCondition.Z,
                max_correction_ratio=MAX_CORRECTION_RATIO
            )
        case Variant.ProjectedTipLossAndLiftingLine:
            # The tip loss only smears the tip vortices in the CFD domain. The forces on the rotor
            # are computed from the uncorrected circulation, and the lifting line correction adds
            # the lift-induced velocities from the uncorrected circulation, so that no additional
            # lift-induced drag should be necessary.
            tip_loss_correction = TipLossCorrection(
                exp_factor = TIP_LOSS_EXP_FACTOR,
                application = TipLossApplication.ProjectedForces
            )
            lifting_line_correction = LiftingLineCorrectionBuilder(
                symmetry_condition=SymmetryCondition.Z,
                max_correction_ratio=MAX_CORRECTION_RATIO,
                include_wing_interactions=False
            )
        case Variant.LiftingLine:
            lifting_line_correction = LiftingLineCorrectionBuilder(
                symmetry_condition=SymmetryCondition.Z,
                max_correction_ratio=MAX_CORRECTION_RATIO
            )
            tip_loss_correction = None
        case Variant.NoCorrections:
            lifting_line_correction = None
            tip_loss_correction = None

    return ActuatorLineBuilder(
        line_force_model=line_force_model,
        projection_settings=ProjectionSettings(
            projection_function=Gaussian(
                chord_factor=0.6,
                thickness_factor=0.6
            ),
        ),
        solver_settings=SolverSettings(damping_factor=0.05),
        sampling_settings=SamplingSettings(remove_span_velocity=True),
        tip_loss_correction=tip_loss_correction,
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
