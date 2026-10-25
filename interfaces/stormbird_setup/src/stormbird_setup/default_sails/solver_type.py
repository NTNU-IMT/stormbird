"""
Copyright (C) 2026, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
"""

from enum import Enum

from ..lifting_line.solver import Linearized as LinearizedSolver, Iterative as IterativeSolver
from ..lifting_line.velocity_corrections import VelocityCorrections

class SolverType(Enum):
    '''
    The lifting line solver that a default sail is set up for. The two solvers give somewhat
    different results around stall, as the linearized solver gives a flatter lift curve that
    stalls at a larger angle of attack. The stall angles, and for some models the stall ranges, of
    the foils in the default sails are therefore different for the two solvers, so that the maximum
    lift, and the angle of attack where it occurs, are about the same.
    '''
    Linearized = "Linearized"
    Iterative = "Iterative"

    def solver(
        self,
        velocity_corrections: VelocityCorrections | None = None
    ) -> LinearizedSolver | IterativeSolver:
        '''
        The solver settings that are used for the default sails
        '''
        if velocity_corrections is None:
            velocity_corrections = VelocityCorrections()

        match self:
            case SolverType.Linearized:
                return LinearizedSolver(velocity_corrections = velocity_corrections)
            case SolverType.Iterative:
                return IterativeSolver(
                    max_iterations_per_time_step = 1000,
                    damping_factor = 0.05,
                    start_with_linearized_solution = True,
                    velocity_corrections = velocity_corrections
                )
            case _:
                raise ValueError("Unsupported solver type:", self)
