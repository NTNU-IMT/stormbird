"""
Copyright (C) 2024, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

Setup of the Stormbird lifting line FMU.
"""

from typing import Self

from pydantic import model_validator

from ..base_model import StormbirdSetupBaseModel
from ..lifting_line import CompleteSailModelBuilder, QuasiSteadySettings


class LiftingLineFmuParameters(StormbirdSetupBaseModel):
    '''
    The parameters file of the Stormbird lifting line FMU, `StormbirdLiftingLine`. The file is
    given to the FMU with the `parameters_path` parameter of the FMU, or as `parameters.json` in the
    resources folder of the FMU.

    Only the sail model is required. See the README of the FMU, or the Stormbird book, for more
    information about each setting.
    '''
    sail_model: CompleteSailModelBuilder
    '''
    The complete sail model, i.e., the lifting line simulation, the wind environment and the
    optional controller. Without a controller, the local wing angles and internal states are
    taken from the inputs of the FMU in every time step.
    '''
    angles_in_degrees: bool = False
    '''Whether the angles in the inputs and outputs of the FMU are given in degrees.'''
    use_motion_velocity: bool = False
    '''
    Whether the motion velocity inputs should replace the velocities computed from the translation
    and rotation inputs with finite differences.
    '''
    motion_velocity_in_body_fixed_frame: bool = False
    '''
    Whether the motion velocity inputs, both linear and angular, are given in the body fixed
    coordinate system, e.g., (u, v, w) and (p, q, r) from a ship simulator.
    '''
    use_motion_velocity_linear_as_freestream: bool = False
    '''
    Whether the linear motion velocity should be part of the apparent wind, rather than motion of
    the sails. Should only be used if the translation input is always zero.
    '''
    input_moving_average_window_size: int = 0
    '''If larger than zero, the motion inputs are filtered with a moving average.'''
    number_of_iterations_before_building_model: int = 0
    '''If larger than zero, the sails are not simulated before this number of time steps.'''
    steady_state: bool = False
    '''
    Whether each time step should be an independent steady state simulation, rather than a step in
    a time-domain simulation. Requires quasi-steady settings for the lifting line simulation.
    '''
    steady_state_optimize_loading: bool = False
    '''
    Whether the steady state simulations should search for the loading with the highest effective
    power, up to the controller loading input. Only allowed together with `steady_state`.
    '''

    @model_validator(mode='after')
    def check_settings(self) -> Self:
        '''
        Checks that the settings are consistent, in the same way as the FMU does when it is set up
        '''
        if self.input_moving_average_window_size < 0:
            raise ValueError("input_moving_average_window_size can not be negative")

        if self.number_of_iterations_before_building_model < 0:
            raise ValueError("number_of_iterations_before_building_model can not be negative")

        if self.steady_state_optimize_loading and not self.steady_state:
            raise ValueError(
                "steady_state_optimize_loading can only be used together with steady_state"
            )

        simulation_settings = self.sail_model.lifting_line_simulation.simulation_settings

        if self.steady_state and not isinstance(simulation_settings, QuasiSteadySettings):
            raise ValueError(
                "The steady state mode of the FMU requires quasi-steady settings for the lifting "
                "line simulation"
            )

        return self
