"""
Copyright (C) 2026, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
"""

import math

import numpy as np

from ..section_models import SectionModel, RotatingCylinder
from ..controller import ControllerSetPoints
from ..lifting_line.velocity_corrections import InducedVelocitySettings, TotalVelocityCorrection

from .sail_definition import SailDefinition
from .solver_type import SolverType

# Based on public data from Norsepower, where a rotor with a diameter of 5 m has a maximum speed of
# 180 RPM
MAX_TIP_SPEED = math.pi * 5.0 * 180.0 / 60.0

def rotor_sail_controller_set_points() -> ControllerSetPoints:
    '''
    Best guess for the controller of a rotor sail, with a maximum spin ratio of 3. The set points are
    given as spin ratio, and are converted to revolutions per second by the controller.
    '''
    return ControllerSetPoints(
        apparent_wind_directions_data = np.radians([-180, -40, -15, 15, 40, 180]).tolist(),
        internal_state_data = [3.0, 3.0, 0.0, 0.0, -3.0, -3.0]
    )

def rotor_sail_induced_velocities() -> InducedVelocitySettings:
    '''
    The high lift coefficients of rotor sails can give large induced velocities, which are therefore
    limited to twice the freestream velocity.
    '''
    return InducedVelocitySettings(
        total_correction = TotalVelocityCorrection.new_max_induced_velocity_magnitude(2.0)
    )

def rotor_sail_deybach_2024(solver_type: SolverType = SolverType.Linearized) -> SailDefinition:
    '''
    Model of a rotor sail that is tuned based on the data in: Deybach, 2024, Aerodynamics of the
    Flettner rotor at high Reynolds number, RINA Wind propulsion conference

    The parameters are fitted to the lift data in the paper. The input power uses the default power
    coefficient in Stormbird. The model has no stall, so the section model is the same for both
    solver types.
    '''
    return SailDefinition(
        section_model = SectionModel(
            model = RotatingCylinder(
                cl_initial_slope = 2.4,
                cl_high_spin_ratio_slope = 0.3,
                cl_slope_transition_spin_ratio = 3.2,
                cl_slope_transition_range = 1.0
            )
        ),
        controller_set_points = rotor_sail_controller_set_points(),
        solver = solver_type.solver(rotor_sail_induced_velocities()),
        max_tip_speed = MAX_TIP_SPEED
    )

def rotor_sail_tillig_2020(solver_type: SolverType = SolverType.Linearized) -> SailDefinition:
    '''
    Model of a rotor sail that is tuned based on the empirical model in: Tillig and Ringsberg, 2020,
    Design, operation and analysis of wind-assisted cargo ships, Ocean Engineering, 211, 107603.

    The empirical model gives the lift, drag and power coefficients of a complete rotor with an
    aspect ratio of 6 and an end disc with twice the diameter of the rotor, as polynomials in the
    spin ratio. It is based on CFD results, corrected to match full-scale measurements, and gives
    considerably higher lift than the other rotor sail models, e.g., a lift coefficient of about 9.5
    at a spin ratio of 3.

    The lift parameters are fitted so that a lifting line simulation of a single rotor with an
    aspect ratio of 6, standing on a symmetry plane, matches the lift polynomial for spin ratios
    between 0 and 5. The polynomial has an S-shape at low spin ratios, which the model can not
    capture, so the lift is overpredicted below a spin ratio of about 1.5.

    The drag is fitted as a function of the lift coefficient, as this is what matters for the
    performance of the sail, and the lift is not the same as in the polynomial for a given spin
    ratio. The end disc is modelled with a virtual extension of the span above the top of the
    rotor, which moves the tip vortex away from the rotor, and therefore reduces the induced drag.
    The length of the extension and the section drag parameters are fitted together, so that the
    drag as a function of the lift matches the polynomials. The drag is within about 10 % for lift
    coefficients above 3, and within a few percent above 9. The minimum in the drag at low lift
    coefficients in the polynomials, about 0.2 at a lift coefficient of about 1, is not captured,
    so the drag is overpredicted there.
    The input power uses the default power coefficient in Stormbird, which is the polynomial from
    the same paper. The model has no stall, so the section model is the same for both solver types.

    See the tillig_empirical_model example in pystormbird for a comparison against the empirical
    model.
    '''
    return SailDefinition(
        section_model = SectionModel(
            model = RotatingCylinder(
                cl_initial_slope = 3.1,
                cl_high_spin_ratio_slope = 0.8,
                cl_slope_transition_spin_ratio = 3.65,
                cl_slope_transition_range = 0.6,
                cd_start = 0.61,
                cd_end = 0.35,
                cd_angle_start = math.radians(56.0),
                cd_spin_ratio_low_drag = 0.2
            )
        ),
        controller_set_points = rotor_sail_controller_set_points(),
        solver = solver_type.solver(rotor_sail_induced_velocities()),
        # Virtual end disc at the top only. The bottom is on the deck or a symmetry plane.
        virtual_extension_factors = (0.0, 0.33),
        max_tip_speed = MAX_TIP_SPEED
    )

def rotor_sail_ostman_2023(solver_type: SolverType = SolverType.Linearized) -> SailDefinition:
    '''
    Model of a rotor sail that is tuned based on data in: Ostman, 2023, Calculation of Flettner rotor
    forces using lifting line and CFD methods, NuTTS conference

    The parameters are fitted to the lift data in the paper. The input power uses the default power
    coefficient in Stormbird. The model has no stall, so the section model is the same for both
    solver types.
    '''
    return SailDefinition(
        section_model = SectionModel(
            model = RotatingCylinder(
                cl_initial_slope = 2.95,
                cl_high_spin_ratio_slope = 0.85,
                cl_slope_transition_spin_ratio = 3.0,
                cl_slope_transition_range = 1.0
            )
        ),
        controller_set_points = rotor_sail_controller_set_points(),
        solver = solver_type.solver(rotor_sail_induced_velocities()),
        max_tip_speed = MAX_TIP_SPEED
    )
