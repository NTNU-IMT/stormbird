"""
Copyright (C) 2026, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
"""

import numpy as np

from ..section_models import SectionModel, Foil, MultiElementFoil
from ..controller import ControllerSetPoints

from .sail_definition import SailDefinition, STALL_ANGLE_MARGIN
from .solver_type import SolverType

def generic_single_element_wing_sail(solver_type: SolverType = SolverType.Linearized) -> SailDefinition:
    '''
    Best guess for a generic single element wing sail. The section model is the default foil model
    in Stormbird, with a stall angle of 20 degrees for the linearized solver. The stall angle for the
    iterative solver is 20.5 degrees, which gives about the same maximum lift, at about the same
    angle of attack, for a sail with an aspect ratio of 7. The maximum effective angle of attack in
    the controller is 5 degrees below the angle of attack with maximum 2D lift, which is 17.26
    degrees for the linearized solver and 17.71 degrees for the iterative solver.

    The angle of attack set points are effective angles of attack, i.e., the angle of attack
    measured on the sail, including the velocities induced by the sail itself and by other sails.
    The controller therefore adjusts the sails until the measured angle of attack matches the set
    point, which ensures that multiple sails are operated close to the set point, also when they
    affect each other.
    '''
    match solver_type:
        case SolverType.Linearized:
            stall_angle = np.radians(20.0)
            max_lift_angle = np.radians(17.26)
        case SolverType.Iterative:
            stall_angle = np.radians(20.5)
            max_lift_angle = np.radians(17.71)

    section_model = SectionModel(
        model = Foil(
            mean_positive_stall_angle = stall_angle,
            mean_negative_stall_angle = stall_angle
        )
    )

    max_angle_of_attack = max_lift_angle - STALL_ANGLE_MARGIN

    controller_set_points = ControllerSetPoints(
        apparent_wind_directions_data = np.radians([-180, -15, -10, 10, 15, 180]).tolist(),
        angle_of_attack_data = [
            -max_angle_of_attack, -max_angle_of_attack, 0.0, 0.0, max_angle_of_attack, max_angle_of_attack
        ],
        use_effective_angle_of_attack = True
    )

    return SailDefinition(
        section_model = section_model,
        controller_set_points = controller_set_points,
        solver = solver_type.solver()
    )

def generic_two_element_wing_sail(solver_type: SolverType = SolverType.Linearized) -> SailDefinition:
    '''
    Best guess for a generic two-element wing sail, i.e., a main element with a flap. The internal
    state is the flap angle. The foil data is only given for positive flap angles, as negative flap
    angles give the mirrored foils. The controller uses a maximum flap angle of 30 degrees, and a
    maximum effective angle of attack that is 5 degrees below the angle of attack with maximum 2D
    lift at this flap angle, which is 11.16 degrees for the linearized solver and 11.64 degrees for
    the iterative solver.

    The stall angles for the linearized solver are 20 degrees without flap, and 15 degrees with a
    flap angle of 30 degrees. The stall angles for the iterative solver are 0.5 degrees larger,
    which gives about the same maximum lift, at about the same angle of attack, for a sail with an
    aspect ratio of 7.

    The angle of attack set points are effective angles of attack, i.e., the angle of attack
    measured on the sail, including the velocities induced by the sail itself and by other sails.
    The controller therefore adjusts the sails until the measured angle of attack matches the set
    point, which ensures that multiple sails are operated close to the set point, also when they
    affect each other.
    '''
    match solver_type:
        case SolverType.Linearized:
            stall_angle_no_flap = np.radians(20.0)
            stall_angle_max_flap = np.radians(15.0)
            max_lift_angle_max_flap = np.radians(11.16)
        case SolverType.Iterative:
            stall_angle_no_flap = np.radians(20.5)
            stall_angle_max_flap = np.radians(15.5)
            max_lift_angle_max_flap = np.radians(11.64)

    section_model = SectionModel(
        model = MultiElementFoil(
            internal_state_data = np.radians([0.0, 30.0]).tolist(),
            foils_data = [
                Foil(
                    cl_zero_angle = 0.0,
                    cd_min = 0.01,
                    mean_positive_stall_angle = stall_angle_no_flap,
                    mean_negative_stall_angle = stall_angle_no_flap
                ),
                Foil(cl_zero_angle = 2.0, cd_min = 0.01, mean_positive_stall_angle = stall_angle_max_flap)
            ]
        )
    )

    max_angle_of_attack = max_lift_angle_max_flap - STALL_ANGLE_MARGIN
    max_flap_angle = np.radians(30.0)

    controller_set_points = ControllerSetPoints(
        apparent_wind_directions_data = np.radians([-180, -15, -10, 10, 15, 180]).tolist(),
        angle_of_attack_data = [
            -max_angle_of_attack, -max_angle_of_attack, 0.0, 0.0, max_angle_of_attack, max_angle_of_attack
        ],
        internal_state_data = [
            -max_flap_angle, -max_flap_angle, 0.0, 0.0, max_flap_angle, max_flap_angle
        ],
        use_effective_angle_of_attack = True
    )

    return SailDefinition(
        section_model = section_model,
        controller_set_points = controller_set_points,
        solver = solver_type.solver()
    )
