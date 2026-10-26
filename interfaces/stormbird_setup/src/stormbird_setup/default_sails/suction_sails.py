"""
Copyright (C) 2026, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
"""

import numpy as np

from ..section_models import (
    SectionModel, Foil, VaryingFoil, InternalFlow, InternalStateConversion, ActiveAirFlowFoil
)
from ..controller import ControllerSetPoints
from ..circulation_corrections import CirculationCorrectionBuilder

from .sail_definition import SailDefinition, STALL_ANGLE_MARGIN
from .solver_type import SolverType

def suction_sail_charrier_1985(solver_type: SolverType = SolverType.Linearized) -> SailDefinition:
    '''
    Model of the original TurboSail suction sail, tuned to the wind tunnel data in: Charrier et al.,
    1985, Fondation Cousteau and windship propulsion 1980 - 1985, System Cousteau - Pechiney, Journal
    of Wind Engineering and Industrial Aerodynamics, 20, 39-60.

    The foil data is given as a function of the suction coefficient, and the internal state
    conversion is PowerToInternalFlow, meaning that the wing internal state is the input power to the
    fan. The data is only given for positive suction coefficients, as negative values give the
    mirrored foils.

    The values are generated with the scripts in the pystormbird example
    "interfaces/pystormbird/examples/suction_sails/turbosail", and are hard coded here:

    - The foils for the suction coefficients 0.027, 0.036 and 0.05 are tuned manually to the 3D lift
      and drag data (compare_3d_lift_and_drag.py).
    - The foil for zero suction is based on the 2D data, with the lift at zero angle of attack
      reduced from 1.8 to 1.4, so that the maximum lift matches the lowest points in the power
      curve. The drag is assumed to be high, as for the other suction coefficients.
    - The 2D data shows that the suction has a small effect on the lift below a suction coefficient
      of 0.018, and a large effect above. A foil is therefore added at this threshold. The stall
      angle (16 degrees) is based on the 2D data, the lift at zero angle of attack is reduced from
      2.18 to 1.5, as for zero suction, and the other parameters are interpolated between the
      foils for zero suction and 0.027.
    - The internal flow model is tuned to the power curve, with the porous fraction and fan
      efficiency fixed, and cp_no_suction fixed at zero (tune_power_model.py).

    The tuning was done with a lifting line model of the wind tunnel model, with an aspect ratio of 4
    and one end against the wind tunnel wall. The lift and drag of other sail geometries will
    therefore include the uncertainty of transferring the 3D tuning to a new geometry. The
    simulation settings from the tuning are part of the definition, as the lift at high suction
    depends on them: Gaussian smoothing of the circulation, and a virtual extension above the top of
    the sail, with a length of 0.4 chord lengths.

    The tuning was done with the iterative solver. The linearized solver gives a flatter lift curve
    around stall, so the stall angles and stall ranges of the foils are reduced for the linearized
    solver. They are tuned so that the maximum lift, and the angle of attack where it occurs, are
    the same as with the iterative solver, for the wind tunnel model with a constant suction
    coefficient along the span. The maximum lift is within 0.3 %, and the angle within 0.75
    degrees, for all suction coefficients in the data. The shape of the lift curve after stall is
    not matched.

    The controller uses a maximum suction coefficient of 0.05, which is the highest suction
    coefficient in the data, and a maximum effective angle of attack that is 5 degrees below the
    angle of attack with maximum 2D lift at this suction coefficient, which is 29.78 degrees for the
    linearized solver and 31.62 degrees for the iterative solver. The suction set points
    are given as suction coefficients, and are converted to the fan power by the controller.

    The angle of attack set points are effective angles of attack, i.e., the angle of attack
    measured on the sail, including the velocities induced by the sail itself and by other sails.
    The controller therefore adjusts the sails until the measured angle of attack matches the set
    point, which ensures that multiple sails are operated close to the set point, also when they
    affect each other.
    '''
    internal_state_data = [0.0, 0.018, 0.027, 0.036, 0.05]

    cl_zero_angle = [1.4, 1.5, 2.5, 3.3, 3.55]
    cl_initial_slope = (np.array([1.08, 1.1933, 1.25, 1.5, 1.65]) * 2 * np.pi).tolist()
    cd_min = [0.4, 0.3333, 0.3, 0.2, 0.2]
    match solver_type:
        case SolverType.Linearized:
            mean_positive_stall_angle = np.radians([15.5, 15.0, 25.5, 27.5, 29.0]).tolist()
            stall_range = np.radians([2.0, 2.0, 2.0, 2.0, 2.0]).tolist()   
            cl_slope_reduction_factor_positive = [0.3, 0.23333, 0.2, 0.3, 0.3]
        case SolverType.Iterative:
            mean_positive_stall_angle = np.radians([17.0, 16.0, 33.0, 36.0, 38.0]).tolist()
            stall_range = np.radians([6.0, 8.6667, 10.0, 10.0, 10.0]).tolist()
            cl_slope_reduction_factor_positive = [0.5, 0.4333, 0.4, 0.5, 0.5]

    max_lift_angle = np.radians(28.0)

    cl_slope_reduction_onset_angle_positive = np.radians([8.0, 12.6667, 15.0, 12.0, 12.0]).tolist()

    foils_data = [
        Foil(
            cl_zero_angle = cl_zero_angle[i],
            cl_initial_slope = cl_initial_slope[i],
            cd_min = cd_min[i],
            cd_second_order_factor = 0.12,
            mean_positive_stall_angle = mean_positive_stall_angle[i],
            mean_negative_stall_angle = float(np.radians(30.0)),
            stall_range = stall_range[i],
            cl_slope_reduction_factor_positive = cl_slope_reduction_factor_positive[i],
            cl_slope_reduction_onset_angle_positive = cl_slope_reduction_onset_angle_positive[i],
        ) for i in range(len(internal_state_data))
    ]

    internal_flow = InternalFlow(
        cp_no_suction = 0.0,
        cp_suction_gradient = -102.08,
        loss_coefficient = 10.345,
        porous_fraction = 0.25,
    )

    section_model = SectionModel(
        model = ActiveAirFlowFoil(
            varying_foil = VaryingFoil(
                internal_state_data = internal_state_data,
                foils_data = foils_data
            ),
            internal_state_conversion = InternalStateConversion.new_power_to_internal_flow(
                internal_flow = internal_flow,
                fan_efficiency = 0.7
            )
        )
    )

    max_angle_of_attack = max_lift_angle - STALL_ANGLE_MARGIN
    max_suction_coefficient = 0.05

    controller_set_points = ControllerSetPoints(
        apparent_wind_directions_data = np.radians([-180, -15, -10, 10, 15, 180]).tolist(),
        angle_of_attack_data = [
            -max_angle_of_attack, -max_angle_of_attack, 0.0, 0.0, max_angle_of_attack, max_angle_of_attack
        ],
        internal_state_data = [
            -max_suction_coefficient, -max_suction_coefficient, 0.0,
            0.0, max_suction_coefficient, max_suction_coefficient
        ],
        use_effective_angle_of_attack = True
    )

    return SailDefinition(
        section_model = section_model,
        controller_set_points = controller_set_points,
        solver = solver_type.solver(),
        circulation_correction = CirculationCorrectionBuilder.new_gaussian_smoothing(0.1),
        virtual_extension_factors = (0.0, 0.4)
    )
