"""
Copyright (C) 2026, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
"""

from enum import Enum

from ..section_models import SectionModel
from ..controller import ControllerSetPoints

from .sail_definition import SailDefinition
from .solver_type import SolverType
from .wing_sails import generic_single_element_wing_sail, generic_two_element_wing_sail
from .rotor_sails import rotor_sail_deybach_2024, rotor_sail_ostman_2023
from .suction_sails import suction_sail_charrier_1985

class DefaultSail(Enum):
    '''
    The default sail models in the library. Each model is either a best guess for a generic sail,
    indicated by the "Generic" prefix, or tuned to a specific source in the literature, indicated by
    the name of the first author and the year. See the function that creates each definition for
    more information about the model.

    Each model is available for both the linearized and the iterative lifting line solver, where the
    linearized solver is the default. The only difference between the two is the stall parameters of
    the foils, i.e., the stall angles and, for some models, the stall ranges, so that the maximum
    lift, and the angle of attack where it occurs, are about the same for both solvers.
    '''
    GenericSingleElementWingSail = "GenericSingleElementWingSail"
    GenericTwoElementWingSail = "GenericTwoElementWingSail"
    RotorSailDeybach2024 = "RotorSailDeybach2024"
    RotorSailOstman2023 = "RotorSailOstman2023"
    SuctionSailCharrier1985 = "SuctionSailCharrier1985"

    def definition(self, solver_type: SolverType = SolverType.Linearized) -> SailDefinition:
        '''
        The complete definition of the sail, for the given solver type, which can be scaled to
        different sizes
        '''
        match self:
            case DefaultSail.GenericSingleElementWingSail:
                return generic_single_element_wing_sail(solver_type)
            case DefaultSail.GenericTwoElementWingSail:
                return generic_two_element_wing_sail(solver_type)
            case DefaultSail.RotorSailDeybach2024:
                return rotor_sail_deybach_2024(solver_type)
            case DefaultSail.RotorSailOstman2023:
                return rotor_sail_ostman_2023(solver_type)
            case DefaultSail.SuctionSailCharrier1985:
                return suction_sail_charrier_1985(solver_type)
            case _:
                raise ValueError("Unsupported default sail:", self)

    def section_model(self, solver_type: SolverType = SolverType.Linearized) -> SectionModel:
        return self.definition(solver_type).section_model

    def controller_set_points(self, chord_length: float) -> ControllerSetPoints:
        '''
        Controller set points for a sail with the given chord length (diameter for rotor sails)
        '''
        return self.definition().scaled_controller_set_points(chord_length)
