"""
Copyright (C) 2026, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
"""

from enum import Enum

from pydantic import Field

from ...base_model import StormbirdSetupBaseModel
from ...spatial_vector import SpatialVector
from ...openfoam.time_control import SteadyTimeControl, DynamicTimeControl
from ...stormflow.pressure_solver import ComputePlatform
from ...stormflow.turbulence import TurbulenceSolverBuilder

from .coordinate_system import ForwardDirection

# Kinematic viscosity of air, used as the molecular viscosity in Stormflow, together with the
# turbulence model.
AIR_KINEMATIC_VISCOSITY = 1.5e-5


class CfdSolver(Enum):
    '''The CFD solver that runs the actuator line simulation'''
    OpenFOAM = "OpenFOAM"
    Stormflow = "Stormflow"


class ShipDomain(StormbirdSetupBaseModel):
    '''
    Size of the computational domain around a ship, with all values in ship lengths. The domain is
    fixed to the ship, and placed relative to the aft and forward perpendiculars along x, the
    centerline along y, and the water plane along z, which is the bottom of the domain.
    '''
    ahead_of_bow: float
    behind_stern: float
    side: float
    """Distance from the centerline to each side of the domain"""
    height: float

    def bounds(
        self,
        ship_length: float,
        ship_x_range: tuple[float, float],
        forward_direction: ForwardDirection
    ) -> tuple[SpatialVector, SpatialVector]:
        '''The start and end point of the domain'''
        x_min, x_max = ship_x_range

        if forward_direction == ForwardDirection.PositiveX:
            extension_at_x_min, extension_at_x_max = self.behind_stern, self.ahead_of_bow
        else:
            extension_at_x_min, extension_at_x_max = self.ahead_of_bow, self.behind_stern

        start = SpatialVector(
            x = x_min - extension_at_x_min * ship_length,
            y = -self.side * ship_length,
            z = 0.0
        )
        end = SpatialVector(
            x = x_max + extension_at_x_max * ship_length,
            y = self.side * ship_length,
            z = self.height * ship_length
        )

        return start, end


class OpenFoamShipSettings(StormbirdSetupBaseModel):
    '''
    Standard settings for OpenFOAM simulations of sails on a ship. The resolutions are given in
    cells per mean chord length of the sails. The mesh is a coarse background mesh, refined around
    the sails and at the surface of the ship.
    '''
    domain: ShipDomain = Field(
        default_factory=lambda: ShipDomain(ahead_of_bow=1.5, behind_stern=1.5, side=1.0, height=1.0)
    )
    background_cells_per_chord: int = 1
    actuator_line_cells_per_chord: int = 16
    actuator_line_refinement_padding: float = 1.0
    """See RefinementBuilder.padding in the OpenFOAM grid"""
    ship_cells_per_chord: int = 4
    time_control: SteadyTimeControl | DynamicTimeControl = Field(default_factory=SteadyTimeControl)


class StormflowShipSettings(StormbirdSetupBaseModel):
    '''
    Standard settings for Stormflow simulations of sails on a ship. The grid is uniform, so the
    domain is smaller than for OpenFOAM. The resolution is given in cells per mean chord length of
    the sails. A RANS turbulence model is used, with the viscosity of air as molecular viscosity.
    '''
    domain: ShipDomain = Field(
        default_factory=lambda: ShipDomain(ahead_of_bow=0.5, behind_stern=1.0, side=0.75, height=0.5)
    )
    cells_per_chord: int = 8
    compute_platform: ComputePlatform = ComputePlatform.GPU
    effective_viscosity: float = AIR_KINEMATIC_VISCOSITY
    turbulence: TurbulenceSolverBuilder | None = Field(default_factory=TurbulenceSolverBuilder)
    courant_number: float = 0.75
    """Courant number used to set the time step"""
    ship_lengths_of_travel: float = 3.0
    """
    Simulated time, as the number of ship lengths the apparent wind at the sails travels. The end
    time is therefore longer for low apparent wind velocities, e.g., in tail wind.
    """
    nr_full_results: int = 6
    """Number of times the full results are stored during the simulation, evenly spaced in time"""


class StormflowRunSettings(StormbirdSetupBaseModel):
    '''Settings for running a Stormflow simulation of one test condition'''
    end_time: float
    courant_number: float
    full_result_interval: float
    """Time between the full results that are stored"""
