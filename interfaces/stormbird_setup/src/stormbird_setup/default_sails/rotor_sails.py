"""
Copyright (C) 2026, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
"""

import math

import numpy as np

from ..section_models import SectionModel, RotatingCylinder
from ..controller import ControllerSetPoints
from ..lifting_line.velocity_corrections import VelocityCorrections

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

def rotor_sail_velocity_corrections() -> VelocityCorrections:
    '''
    The high lift coefficients of rotor sails can give large induced velocities, which are therefore
    limited to twice the freestream velocity.
    '''
    return VelocityCorrections.new_max_induced_velocity_magnitude(2.0)

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
        solver = solver_type.solver(rotor_sail_velocity_corrections()),
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
        solver = solver_type.solver(rotor_sail_velocity_corrections()),
        max_tip_speed = MAX_TIP_SPEED
    )
